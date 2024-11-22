use age::{secrecy::SecretString, Decryptor};
use anyhow::{anyhow, bail, Result};
use async_std::task::sleep;
use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use file_format::FileFormat;
use futures_util::AsyncReadExt;
use futures_util::{io, StreamExt};
use reqwest::Client;
use std::{fmt::Display, sync::OnceLock, time::Duration};

// Urls are relative to your Cargo.toml file
const _TAILWIND_URL: &str = manganis::mg!(file("public/tailwind.css"));

const ASSET: manganis::ImageAsset = manganis::mg!(image("assets/logo.png"));

// Define a global signal holding the action state
enum Action {
    Idle,
    Downloading,
    PreparingDecryption,
    Decrypting,
    FinishingFile,
}

static ACTION_IN_PROGRESS: GlobalSignal<Action> = Signal::global(|| Action::Idle);

static BACKEND_URL: OnceLock<String> = OnceLock::new();

fn main() {
    // We unwrap since without this we can't actually use our frontend
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut href = location.origin().unwrap();
    if href.ends_with('/') {
        href = href.trim_end_matches('/').to_string();
    }

    BACKEND_URL
        .set(href)
        .map_err(|err| anyhow!("Could not set oncelock: {err}"))
        .unwrap();

    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Idle => f.write_str(""),
            Action::Downloading => f.write_str("Downloading..."),
            Action::PreparingDecryption => f.write_str("Decrypting is about to start..."),
            Action::Decrypting => f.write_str("Decrypting..."),
            Action::FinishingFile => f.write_str("Finishing..."),
        }
    }
}

async fn get_file(id: String, pwd: String) -> Result<(String, Vec<u8>)> {
    if id.is_empty() {
        bail!("File name cannot be empty.")
    }
    // Set the action to downloading
    *ACTION_IN_PROGRESS.write() = Action::Downloading;

    let query = vec![("file", &id)];
    let res = Client::new()
        // Here it's safe to unwrap since we know for sure it's initialized
        .get(format!("{}/api/download", BACKEND_URL.get().unwrap()))
        .query(&query)
        .send()
        .await?;

    // Checks if there's been an error.
    if !res.status().is_success() {
        let status = res.status().to_string();
        return Err(anyhow!("Status: {status}"));
    }

    // Gets the filename from headers.
    let file_name = match res.headers().get("filecrab-file-name") {
        Some(file_name) => file_name.to_str()?.to_string(),
        None => return Err(anyhow!("Could not get filename from response header.")),
    };

    // Inits the stream.
    let mut stream = res.bytes_stream();

    // Creates the buffer.
    let mut buf: Vec<u8> = Vec::new();

    // Reads the stream.
    while let Some(data) = stream.next().await {
        let chunk = data?;
        io::copy(&mut chunk.as_ref(), &mut buf).await?;
    }

    *ACTION_IN_PROGRESS.write() = Action::Idle;
    sleep(Duration::from_millis(10)).await;

    let is_encrypted = FileFormat::from_bytes(&buf) == FileFormat::AgeEncryption;

    // If file is encoded try to decrypt it
    if is_encrypted && pwd.is_empty() {
        bail!("You must provide a password for this file as it is encrypted")
    } else if is_encrypted {
        *ACTION_IN_PROGRESS.write() = Action::PreparingDecryption;
        sleep(Duration::from_millis(10)).await;

        let output = decrypt_slice(&buf[..], pwd).await?;
        return Ok((file_name, output));
    }

    // Simply return the file
    Ok((file_name, buf))
}

/// Given a slice of bytes and a password, tries to decrypt it's values and returns the
/// original content.
/// Uses the age algorithm.
async fn decrypt_slice(buf: &[u8], pwd: String) -> anyhow::Result<Vec<u8>> {
    let decryptor = Decryptor::new_async_buffered(buf).await?;
    let mut output = vec![];
    let mut reader = decryptor.decrypt_async(std::iter::once(&age::scrypt::Identity::new(
        SecretString::from(pwd),
    ) as _))?;

    // Set the action to decrypting
    *ACTION_IN_PROGRESS.write() = Action::Decrypting;
    sleep(Duration::from_millis(10)).await;

    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => {
                break;
            }
            Ok(n) => {
                output.extend_from_slice(&buffer[..n]);
                sleep(Duration::from_millis(1)).await;
            }
            Err(err) => {
                bail!(err)
            }
        }
    }
    // Restore the in progress to idle after finishing the task
    *ACTION_IN_PROGRESS.write() = Action::FinishingFile;
    sleep(Duration::from_millis(50)).await;

    Ok(output)
}

async fn fetch_file(id: String, pwd: String) -> Result<()> {
    let (file_name, data) = get_file(id, pwd).await?;

    let new_eval = eval(
        r#"
        let filename = await dioxus.recv();
        let content = await dioxus.recv();

        // Convert from uint8array
        var contentBytes = new Uint8Array(content);

        var contentType = 'application/octet-stream';
        var a = document.createElement('a');
        var blob = new Blob([contentBytes], {'type':contentType});
        a.setAttribute("download", filename);
        a.setAttribute("href", window.URL.createObjectURL(blob));
        a.click();
        "#,
    );

    // Convert the Vec<u8> to serde::Value using serde_bytes::ByteBuf
    let data_value = serde_bytes::ByteBuf::from(data);

    new_eval
        .send(file_name.into())
        .map_err(|err| anyhow!("{err:?}").context("could not eval filename"))?;
    new_eval
        .send(serde_json::to_value(data_value)?)
        .map_err(|err| anyhow!("{err:?}").context("could not eval data"))?;

    Ok(())
}

#[component]
fn App() -> Element {
    let id = use_signal(|| "".to_string());
    let pwd = use_signal(|| "".to_string());

    let fetch_result = use_signal(|| None::<anyhow::Result<()>>);

    // Boolean to control the fact of showing or not the error toast
    let mut show = use_signal_sync(|| true);

    use_effect(move || {
        if let Some(Err(_)) = &*fetch_result.read() {
            show.set(true);
        }
    });

    rsx! {
        div { class: "container mx-auto max-w-screen-xl px-6",
            img { class: "mx-auto", src: "{ASSET}" },
            DownloadForm { id, pwd, fetch_result }
            {
                match &*ACTION_IN_PROGRESS.read() {
                    Action::Idle => {
                        None
                    },
                    Action::PreparingDecryption | Action::FinishingFile => {
                        rsx!{
                            div {
                                class: "flex justify-center items-center flex-col gap-2 pt-8",
                                p {
                                    "{ACTION_IN_PROGRESS}"
                                }
                            }
                        }
                    }
                    _ => {
                        rsx!{
                            div {
                                class: "flex justify-center items-center flex-col gap-2 pt-8",
                                span {
                                    class: "loading loading-spinner loading-lg text-primary"
                                }
                                p {
                                    "{ACTION_IN_PROGRESS}"
                                }
                            }
                        }
                    }
                }
            }
            {
                match &*fetch_result.read() {
                    Some(Ok(_)) => {
                        None
                    },
                    Some(Err(err)) => {
                        rsx!{
                                ErrorToast {
                                    err: err.to_string(),
                                    show: show,
                                }
                        }
                    },
                    None => {None},
                }
            }
        }
    }
}

#[component]
fn DownloadForm(
    id: Signal<String>,
    pwd: Signal<String>,
    fetch_result: Signal<Option<Result<()>>>,
) -> Element {
    // Button is disabled unless there is something to sed
    let mut disable_button = use_signal(|| true);
    let mut disable_form = use_signal(|| false);
    use_effect(move || {
        if !id().is_empty() {
            disable_button.set(false);
        } else {
            disable_button.set(true);
        }
    });

    rsx! {
        form {
            onsubmit: move |_| {
                spawn(async move {
                    // Disable button and form
                    disable_form.set(true);
                    disable_button.set(true);

                    // Fetch file
                    let result = fetch_file(id(), pwd()).await;

                    // If the result is ok, clear the form
                    if result.is_ok() {
                        id.set("".to_string());
                        pwd.set("".to_string());
                    }

                    // Set the result response
                    fetch_result.set(Some(result));

                    // Update app state
                    *ACTION_IN_PROGRESS.write() = Action::Idle;
                    sleep(Duration::from_millis(50)).await;

                    // Re enable button and form
                    disable_button.set(false);
                    disable_form.set(false);
                });
            },
            class: "max-w-md mx-auto gap-4 flex flex-col",
            label { class: "input input-bordered input-primary flex items-center gap-2",
                // Document svg
                svg {
                    height: "16px",
                    stroke_width: "1.5",
                    width: "16px",
                    fill: "none",
                    xmlns: "http://www.w3.org/2000/svg",
                    color: "currentColor",
                    view_box: "0 0 24 24",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke: "currentColor",
                        d: "M4 21.4V2.6C4 2.26863 4.26863 2 4.6 2H16.2515C16.4106 2 16.5632 2.06321 16.6757 2.17574L19.8243 5.32426C19.9368 5.43679 20 5.5894 20 5.74853V21.4C20 21.7314 19.7314 22 19.4 22H4.6C4.26863 22 4 21.7314 4 21.4Z",
                        stroke_width: "1.5"
                    }
                    path {
                        stroke_linecap: "round",
                        stroke_width: "1.5",
                        d: "M8 10L16 10",
                        stroke_linejoin: "round",
                        stroke: "currentColor"
                    }
                    path {
                        stroke_width: "1.5",
                        d: "M8 18L16 18",
                        stroke: "currentColor",
                        stroke_linecap: "round",
                        stroke_linejoin: "round"
                    }
                    path {
                        d: "M8 14L12 14",
                        stroke_linejoin: "round",
                        stroke_linecap: "round",
                        stroke: "currentColor",
                        stroke_width: "1.5"
                    }
                    path {
                        stroke_linejoin: "round",
                        d: "M16 2V5.4C16 5.73137 16.2686 6 16.6 6H20",
                        stroke: "currentColor",
                        stroke_width: "1.5",
                        stroke_linecap: "round"
                    }
                }
                input {
                    oninput: move |event| id.set(event.value()),
                    class: "min-w-[90%]",
                    r#type: "text",
                    placeholder: "File id",
                    value: "{id}",
                    disabled: disable_form()
                }
            }
            label { class: "input input-bordered input-primary flex items-center gap-2",
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    view_box: "0 0 16 16",
                    fill: "currentColor",
                    class: "h-4 w-4 opacity-70",
                    path { d: "M14 6a4 4 0 0 1-4.899 3.899l-1.955 1.955a.5.5 0 0 1-.353.146H5v1.5a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1-.5-.5v-2.293a.5.5 0 0 1 .146-.353l3.955-3.955A4 4 0 1 1 14 6Zm-4-2a.75.75 0 0 0 0 1.5.5.5 0 0 1 .5.5.75.75 0 0 0 1.5 0 2 2 0 0 0-2-2Z" }
                }
                input {
                    oninput: move |event| pwd.set(event.value()),
                    class: "grow",
                    r#type: "password",
                    placeholder: "Set your password",
                    value: "{pwd}",
                    disabled: disable_form()
                }
                span {
                    class: "badge badge-ghost",
                    "Optional"
                }
            }
            button {
                class: "btn btn-primary",
                r#type: "submit",
                disabled: disable_button(),
                "Download",
            }
        }
    }
}

#[component]
fn ErrorToast(err: String, show: Signal<bool, SyncStorage>) -> Element {
    if *show.read() {
        // start a thread with 5 seconds
        spawn(async move {
            async_std::task::sleep(Duration::from_secs(3)).await;
            show.set(false);
        });

        rsx! {
            div {
                class: "toast toast-top toast-end",
                div {
                    class: "alert alert-error",
                    svg {
                        "viewBox": "0 0 24 24",
                        "fill": "none",
                        "xmlns": "http://www.w3.org/2000/svg",
                        class: "h-6 w-6 shrink-0 stroke-current",
                        path {
                            "d": "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z",
                            "stroke-linejoin": "round",
                            "stroke-linecap": "round",
                            "stroke-width": "2"
                        }
                    }
                    span { "Error! {err}." }
                }
            }
        }
    } else {
        None
    }
}
