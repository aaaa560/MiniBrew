use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    for (i, arg) in args.iter().enumerate() {
        println!("\nargs[{}] = {}", i, arg)
    }

    if args.len() < 2 {
        println!("Uso: minibrew install <progama>");
        return;
    }

    match args[1].as_str() {
        "install" => {
            if args.len() < 3 {
                println!("Especidique o programa para instalar");
                return;
            }
            let program = &args[2];
            install(program);
        }
        _ => println!("Comando não reconhecido."),
    }
}

fn install(program: &str) {
    println!("Instalando {}...", program);

    let status = match program {
        "python" => Command::new("bash")
            .arg("-c")
            .arg("brew install python")
            .status()
            .expect("Erro ao instalar Pyhton"),
        "rust" => Command::new("bash")
            .arg("-c")
            .arg("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
            .status()
            .expect("Erro ao intalar Rust"),
        "transformers" => Command::new("bash")
            .arg("-c")
            .arg("pip3 install --upgrade pip && pip3 install transformers datasets torch")
            .status()
            .expect("Erro ao installar Transformers"),
        _ => {
            print!("Programa não suportado: {}", program);
            return;
        }
    };

    if status.success() {
        print!("{} instalado com sucesso!", program);
    } else {
        print!("Falha ao instalar {}", program);
    }
}
