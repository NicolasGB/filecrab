use age::{secrecy::Secret, Decryptor};
use anyhow::Result;
use dioxus::prelude::*;
use dioxus_logger::tracing::{debug, info, Level};
use futures_util::{io, StreamExt};
use reqwest::Client;
use std::{io::Read, time::Duration};

// Urls are relative to your Cargo.toml file
const _TAILWIND_URL: &str = manganis::mg!(file("public/tailwind.css"));

const ASSET: manganis::ImageAsset = manganis::mg!(image("assets/logo.png"));

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

async fn get_file(id: String, pwd: String) -> Result<Vec<u8>> {
    debug!("fetching: {id}, {pwd}");
    let query = vec![("file", &id)];
    let res = Client::new()
        .get("http://127.0.0.1:8080/api/download".to_string())
        //WARN: Remove me before commit
        .query(&query)
        .send()
        .await?;

    // Inits the stream.
    let mut stream = res.bytes_stream();

    // Creates the buffer.
    let mut buf: Vec<u8> = Vec::new();

    // Reads the stream.
    while let Some(data) = stream.next().await {
        let chunk = data?;
        io::copy(&mut chunk.as_ref(), &mut buf).await?;
    }

    let output = decrypt_slice(&buf[..], pwd)?;

    Ok(output)
}

/// Given a slice of bytes and a password, tries to decrypt it's values and returns the
/// original content.
/// Uses the age algorithm.
fn decrypt_slice(buf: &[u8], pwd: String) -> anyhow::Result<Vec<u8>> {
    let decryptor = match Decryptor::new(buf)? {
        Decryptor::Passphrase(decryptor) => decryptor,
        _ => unreachable!(),
    };
    let mut output = vec![];
    let mut reader = decryptor.decrypt(&Secret::new(pwd), None)?;
    reader.read_to_end(&mut output)?;
    Ok(output)
}

async fn fetch_file(id: String, pwd: String) -> Result<()> {
    let data = get_file(id, pwd).await?;
    let create_eval = eval(
        r#"
        let filename = await dioxus.recv();
        let content = await dioxus.recv();

        var contentType = 'application/octet-stream';
        var a = document.createElement('a');
        var blob = new Blob([content], {'type':contentType});
        a.href = window.URL.createObjectURL(blob);
        a.download = filename;
        a.click();
        "#,
    );

    create_eval.send("filename".into()).expect("filename");
    create_eval.send(data.into()).expect("data");
    Ok(())
}

#[component]
fn App() -> Element {
    let id = use_signal(|| "".to_string());
    let pwd = use_signal(|| "".to_string());
    let fetch_result = use_signal(|| None::<anyhow::Result<()>>);
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
            div {
                class: "mx-auto",
                {
                    match &*fetch_result.read() {
                        Some(Ok(_)) => {info!("ok"); None},
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
}

#[component]
fn DownloadForm(
    id: Signal<String>,
    pwd: Signal<String>,
    fetch_result: Signal<Option<Result<()>>>,
) -> Element {
    rsx! {
        form {
            onsubmit: move |_| {
                spawn(async move {
                    let result = fetch_file(id(), pwd()).await;
                    fetch_result.set(Some(result));
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
                    r#type: "text",
                    placeholder: "File id",
                    value: "{id}"
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
                    class: "min-w-[90%]",
                    r#type: "password",
                    placeholder: "Set your password"
                }
            }
            input {
                class: "btn btn-primary",
                r#type: "submit",
                value: "Download"
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
                // role: "alert",
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
