use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Uso: minibrew install <progama>");
        return;
    }

    match args[1].as_str() {
        "intall" => {
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

    let status = Command::new("bash")
        .arg("-c")
        .arg(format!("echo Aqui você instalaria {}", program))
        .status()
        .expect("Falha ao executar o comando");

    if status.success() {
        print!("{} instalado com sucesso!", program);
    } else {
        print!("Falha ao instalar {}", program);
    }
}
