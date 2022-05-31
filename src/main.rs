use std::io::Read;
use std::net::{TcpListener, ToSocketAddrs};
use std::sync::mpsc;
use std::thread::spawn;

type GenericResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn in_path(program: &str) -> GenericResult<bool> {
    for path in std::env::var("PATH")?.split(":") {
        let path = std::path::Path::new(path);
        if std::fs::metadata(path.join(program)).is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn init_mpv(resolution: u64, options: Option<&[(&str, &str)]>) -> GenericResult<mpv::MpvHandler> {
    let mut builder = mpv::MpvHandlerBuilder::new()?;
    builder.try_hardware_decoding()?;
    builder.set_option("keep-open", "yes")?;
    builder.set_option("idle", "yes")?;
    builder.set_option("ytdl", "yes")?;
    builder.set_option(
        "ytdl-format",
        format!("bestvideo[height={}]+bestaudio", resolution).as_str(),
    )?;
    if in_path("yt-dlp")? {
        builder.set_option("script-opts", "ytdl_hook-ytdl_path=yt-dlp")?;
    }
    builder.set_option("force-window", "immediate")?;
    builder.set_option("osc", "yes")?;
    builder.set_option("prefetch-playlist", "yes")?;
    if let Some(options) = options {
        for (key, value) in options {
            builder.set_option(key, *value)?;
        }
    }
    Ok(builder.build()?)
}

fn tcp_listener<A: ToSocketAddrs>(addr: A, tx: mpsc::Sender<String>) -> GenericResult<()> {
    let listener = TcpListener::bind(addr)?;
    let buf: &mut [u8] = &mut [0; 1024];
    loop {
        let (mut con, _) = listener.accept()?;
        let mut string = String::new();
        while let Ok(size) = con.read(buf) {
            if size == 0 {
                break;
            }
            if let Ok(s) = std::str::from_utf8(&buf[..size]) {
                string.push_str(s);
            }
        }
        tx.send(string)?;
    }
}

fn main() -> GenericResult<()> {
    let mut handler = init_mpv(720, None)?;

    let (tx, rx) = mpsc::channel();

    spawn(move || {
        tcp_listener("0.0.0.0:8000", tx).unwrap();
    });

    loop {
        if let Some(event) = handler.wait_event(1.0) {
            println!("{:?}", event);
        }
        if let Ok(url) = rx.try_recv() {
            handler.command(&["loadfile", &url, "append-play"])?;
        }
    }
}
