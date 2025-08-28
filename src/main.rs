use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Uso: minibrew <comando> [progama]");
        print!("Comandos: install, uninstall, list, version");
        return;
    }

    let comando = &args[1];
    let programas: Vec<String> = args.iter().skip(2).map(|s| s.to_string()).collect();

    match comando.as_str() {
        "install" => {
            if programas.is_empty() {
                println!("Especifique pelo menos um programa para instalar!");
            } else {
                for prog in programas {
                    install(&prog);
                }
            }
        }
        "uninstall" => {
            if programas.is_empty() {
                print!("Especifique pelo menos um programa para desistalar!");
            } else {
                for prog in programas {
                    uninstall(&prog);
                }
            }
        }
        "list" => {
            print!("Programas suportados: python, rust, transformers");
        }
        "version" => {
            for prog in programas {
                show_version(&prog);
            }
        }
        _ => print!("Comando não reconhecido"),
    }
}

fn install(program: &str) {
    let program = match program {
        "py" => "python",
        "tf" => "transformers",
        other => other,
    };

    println!("Instalando {}...", program);

    let status = match program {
        "python" => Command::new("bash")
            .arg("-c")
            .arg("brew install python")
            .status(),
        "rust" => Command::new("bash")
            .arg("-c")
            .arg("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
            .status(),
        "transformers" => Command::new("bash")
            .arg("-c")
            .arg("pip3 install --upgrade pip && pip3 install transformers datasets torch")
            .status(),
        _ => {
            print!("Programa não suportado: {}", program);
            return;
        }
    };

    match status {
        Ok(s) if s.success() => println!("{} instalado com sucesso!", program),
        _ => println!("Falha ao instalar {}", program),
    }
}

fn uninstall(program: &str) {
    let program = match program {
        "py" => "python",
        "tf" => "transformers",
        other => other,
    };

    println!("Desinstalando {}...", program);

    let status = match program {
        "python" => Command::new("brew").arg("uninstall").arg("python").status(),
        "rust" => Command::new("rustup")
            .arg("self")
            .arg("uninstall")
            .arg("-y")
            .status(),
        "transformers" => Command::new("pip3")
            .arg("uninstall")
            .arg("-y")
            .arg("transformers")
            .status(),
        _ => {
            println!("Programa não suportado: {}", program);
            return;
        }
    };

    match status {
        Ok(s) if s.success() => println!("{} desinstalado com sucesso!", program),
        _ => println!("Falha ao desinstalar {}", program),
    }
}

fn show_version(program: &str) {
    let program = match program {
        "py" => "python",
        "tf" => "transformers",
        other => other,
    };

    let cmd = match program {
        "python" => Command::new("python3").arg("--version").output(),
        "rust" => Command::new("rustc").arg("--version").output(),
        "transformers" => Command::new("pip3")
            .arg("show")
            .arg("transformers")
            .output(),
        _ => {
            println!("Programa não suportado: {}", program);
            return;
        }
    };

    match cmd {
        Ok(out) => println!("{}", String::from_utf8_lossy(&out.stdout)),
        Err(_) => println!("Não foi possivel pegar a versão de {}", program),
    }
}
