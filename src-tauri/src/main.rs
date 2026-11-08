// Entry point delegates to the library so integration tests can reuse it.

fn main() {
    // Privileged mode: pkexec starts this same program again, under the polkit
    // action of the package, to do what needs the administrator. It has to be
    // checked before Tauri starts, because it never opens a window.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--run-ops" {
        std::process::exit(soplos_look_manager_lib::run_ops_file(&args[2]));
    }

    // WebKitGTK's DMA-BUF renderer causes protocol errors (Error 71) on Wayland
    // with NVIDIA/Nouveau drivers on physical hardware, as well as blank windows
    // on virtual GPUs. Force it disabled by default on all hardware.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    soplos_look_manager_lib::run();
}
