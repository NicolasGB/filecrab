<div align="center">
  <img src="logo.png" alt="logo" />
  <h3>⚡ A blazingly fast file and text sharing service ⚡</h3>

  ![Build](https://img.shields.io/github/actions/workflow/status/NicolasGB/filecrab/pipeline.yml?branch=main)
  ![Rust](https://img.shields.io/badge/rust-1.70+-blueviolet.svg?logo=rust)
  ![License](https://img.shields.io/badge/license-MIT-blue.svg)
</div>

<div>
    <h3>Description</h3>
    <p>
        You can host your own instance, simply need a MinIO bucket and a surrealdb instance.<br>
        A useful cli will allow you to upload files and text to your instance. <br>
        The server can be fully configurated with environment variables. <br>
    </p>
    <h3>Features</h3>
    <ul>
        <li>File sharing.</li>
        <li>File expiration.</li>
        <li>One-time text sharing.</li>
        <li>Files <strong>optionally</strong> encrypted.</li>
        <li>Text <strong>always</strong> encrypted.</li>
        <li>Cleanup of expired files server side via a command executable on the server, for example with a cron job.</li>
        <li>Memorable words lists as ids, inspired by <a href="https://github.com/magic-wormhole/magic-wormhole.rs">magic-wormhole</a>.</li>
    </ul>
    <h4>Encryption</h4>
    <p>
    All data is encrypted through the <a href="https://github.com/str4d/rage">age</a> library.<br>
    Encryption is done <strong>client side</strong>, in the cli tool, this allows us to stream files directly to the storage without the need of reading it in memory on the server.<br>
    The password is <strong>never</strong> sent to the server.
    </p>
</div>



