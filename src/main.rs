use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use indicatif::{ProgressBar, ProgressStyle};
use open;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::time::SystemTime;
use ureq;

#[derive(Parser)]
#[command(name = "MiniBrew")]
#[command(about = "Instalador/gerenciador minimal (prototype)")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageEntry {
    name: String,
    mac: Option<String>,
    linux: Option<String>,
    windows: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        package: String,
    },
    Uninstall {
        package: String,
    },
    Update {
        package: String,
    },
    UpgradeAll,
    Stack {
        stack: String,
    },
    SelfUpdate,
    History,
    Undo,
    Config,
    Version,
    Doctor,
    Search {
        query: String,
    },
    Add {
        name: String,
        linux: String,
        mac: String,
        windows: String,
    },
    Info {
        package: String,
    },
    List,
    Export,
    Import {
        file: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    packages: Vec<PackageEntry>,
    stacks: serde_json::Map<String, serde_json::Value>,
    repo_taps: Vec<String>,
    ai_hook: Option<String>,
}

fn ascii_banner() {
    println!(
        r#"MiniBrew v0.3 - "Um debug por vez!"
        _   _ _       _            _
       | \ | (_)     (_)          | |
       |  \| |_ _ __  _  ___ _ __ | |
       | . ` | | '_ \| |/ _ \ '_ \| |
       | |\  | | | | | |  __/ | | |_|
       |_| \_|_|_| |_|_|\___|_| |_(_)
       "install fast. Mini Brew."
       "#,
    );
}

fn project_paths() -> (PathBuf, PathBuf) {
    if let Some(pd) = ProjectDirs::from("com", "minibrew", "MiniBrew") {
        let cfg_dir = pd.config_dir().to_path_buf();
        let data_dir = pd.data_dir().to_path_buf();
        (cfg_dir, data_dir)
    } else {
        (PathBuf::from("."), PathBuf::from("."))
    }
}

fn ensure_dirs() -> io::Result<()> {
    let (cfg_dir, data_dir) = project_paths();
    fs::create_dir_all(&cfg_dir)?;
    fs::create_dir_all(&data_dir)?;
    Ok(())
}

fn config_path() -> PathBuf {
    let (cfg_dir, _) = project_paths();
    cfg_dir.join("config.json")
}

fn log_path() -> PathBuf {
    let (_, data_dir) = project_paths();
    data_dir.join("history.log")
}

fn default_config() -> Config {
    let pkgs = vec![
        PackageEntry {
            name: "python".into(),
            mac: Some("brew install python".into()),
            linux: Some("sudo apt update && sudo apt install -y python3 python3-pip".into()),
            windows: Some("winget install --exact Python.Python".into()),
        },
        PackageEntry {
            name: "rust".into(),
            mac: Some(
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y".into(),
            ),
            linux: Some(
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y".into(),
            ),
            windows: Some("winget install --exact rust-lang.rust".into()),
        },
    ];
    let mut stacks = serde_json::Map::new();
    stacks.insert(
        "ai".into(),
        serde_json::json!(["python", "rust", "transformers"]),
    );
    Config {
        packages: pkgs,
        stacks,
        repo_taps: vec![],
        ai_hook: None,
    }
}

fn read_or_create_config() -> Config {
    let p = config_path();
    if !p.exists() {
        if let Err(e) = ensure_dirs() {
            eprintln!("Erro criando dirs: {}", e);
        }
        let cfg = default_config();
        if let Ok(json) = serde_json::to_string_pretty(&cfg) {
            if let Err(e) = fs::write(&p, json) {
                eprintln!("Falha ao salvar config: {}", e);
            }
        }
        cfg
    } else {
        let mut s = String::new();
        if let Ok(mut f) = File::open(&p) {
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(cfg) = serde_json::from_str::<Config>(&s) {
                    return cfg;
                }
            }
        }
        default_config()
    }
}

fn log_action(action: &str) {
    let p = log_path();
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
        .unwrap();
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    writeln!(f, "{} {}", ts, action).ok();
}

fn last_action() -> Option<String> {
    let p = log_path();
    if !p.exists() {
        return None;
    }
    let content = fs::read_to_string(p).ok()?;
    for line in content.lines().rev() {
        if !line.trim().is_empty() {
            return Some(line.to_string());
        }
    }
    None
}

fn run_shell(cmd: &str) -> bool {
    println!("> {}", cmd);
    if cfg!(target_os = "windows") {
        let status = Command::new("powershell").arg("-Command").arg(cmd).status();
        match status {
            Ok(s) => s.success(),
            Err(_) => false,
        }
    } else {
        let status = Command::new("sh").arg("-c").arg(cmd).status();
        match status {
            Ok(s) => s.success(),
            Err(_) => false,
        }
    }
}

fn platform_command(cfg: &Config, package: &str) -> Option<String> {
    let pkg = cfg.packages.iter().find(|p| p.name == package)?;
    let os = std::env::consts::OS;
    let cmd = match os {
        "macos" => pkg.mac.clone(),
        "linux" => pkg.linux.clone(),
        "windows" => pkg.windows.clone(),
        _ => None,
    };
    cmd
}

fn find_package_command(cfg: &Config, package: &str) -> Option<String> {
    platform_command(cfg, package)
}

fn install_package(cfg: &Config, package: &str) {
    ascii_banner();
    println!("Instalando {}...", package);
    if let Some(cmd) = find_package_command(cfg, package) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("/|\\-"),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_message("executando comando...");
        let ok = run_shell(&cmd);
        pb.finish_and_clear();
        if ok {
            println!("{} instalado com sucesso!", package);
            log_action(&format!("INSTALL {}", package));
        } else {
            println!("Falha ao instalar {}.", package);
            log_action(&format!("FAIL_INSTALL {}", package));
        }
    } else {
        println!("Pacote {} não conhecido no config.", package);
    }
}

fn uninstall_package(cfg: &Config, package: &str) {
    println!("Tentando desinstalar {}.", package);
    let os = std::env::consts::OS;
    let cmd = match os {
        "macos" => format!("brew uninstall {}", package),
        "linux" => format!("sudo apt remove -y {}", package),
        "windows" => format!("winget uninstall {}", package),
        _ => format!("echo 'uninstall not supported on {}'", os),
    };
    let ok = run_shell(&cmd);
    if ok {
        println!("{} desinstalado.", package);
        log_action(&format!("UNINSTALL {}", package));
    } else {
        println!("Falha ao desinstalar {}.", package);
        log_action(&format!("FAIL_UNINSTALL {}", package));
    }
}

fn update_package(cfg: &Config, package: &str) {
    println!("Atualizando {}...", package);
    let os = std::env::consts::OS;
    let cmd = match os {
        "macos" => format!("brew upgrade {}", package),
        "linux" => format!(
            "sudo apt update && sudo apt install --only-upgrade -y {}",
            package
        ),
        "windows" => format!("winget upgrade --id {}", package),
        _ => format!("echo 'update not supported on {}'", os),
    };
    let ok = run_shell(&cmd);
    if ok {
        println!("{} atualizado.", package);
        log_action(&format!("UPDATE {}", package));
    } else {
        println!("Falha ao atualizar {}.", package);
        log_action(&format!("FAIL_UPDATE {}", package));
    }
}

fn upgrade_all(cfg: &Config) {
    println!("Atualizando repositórios e pacotes (all)...");
    let os = std::env::consts::OS;
    let cmd = match os {
        "macos" => "brew update && brew upgrade".to_string(),
        "linux" => "sudo apt update && sudo apt upgrade -y".to_string(),
        "windows" => "winget upgrade --all".to_string(),
        _ => "echo 'upgrade-all not supported'".to_string(),
    };
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message("upgrading...");
    let ok = run_shell(&cmd);
    pb.finish_and_clear();
    if ok {
        println!("Upgrade all finalizado.");
        log_action("UPGRADE_ALL");
    } else {
        println!("Falha no upgrade all.");
        log_action("FAIL_UPGRADE_ALL");
    }
}

fn install_stack(cfg: &Config, stack: &str) {
    if let Some(val) = cfg.stacks.get(stack) {
        if let Some(arr) = val.as_array() {
            println!("Instalando stack '{}' ({} pacotes)...", stack, arr.len());
            for v in arr {
                if let Some(pkg) = v.as_str() {
                    install_package(cfg, pkg);
                }
            }
            log_action(&format!("STACK {}", stack));
        } else {
            println!("Stack inválida no config");
        }
    } else {
        println!("Stack '{}' não encontrada.", stack)
    }
}

fn self_update() {
    println!("Tentando self-update (placeholder)...");
    let url = "https://github.com/aaaa560/MiniBrew/releases/latest/download/minibrew-linux";
    println!("(ex) baixando de: {}", url);

    match ureq::get(url).call() {
        Ok(resp) => {
            if resp.status() == 200 {
                let mut out = Vec::new();
                if resp.into_reader().read_to_end(&mut out).is_ok() {
                    println!("Baixando (X bytes) - salvaria e substituiria o binario.");
                    log_action("SELF_UPDATE");
                }
            } else {
                println!("Resposta http: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Erro ao baixar {}", e);
        }
    }
}

fn show_history() {
    let p = log_path();
    if p.exists() {
        if let Ok(s) = fs::read_to_string(p) {
            println!("———— History ————");
            println!("{}", s);
        }
    } else {
        println!("Nenhum histórico encontrado.");
    }
}

fn undo_last() {
    if let Some(line) = last_action() {
        println!("Última ação: {}", line);
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let op = parts[1];
            if line.contains("INSTALL") {
                let cfg = read_or_create_config();
                uninstall_package(&cfg, op);
                log_action(&format!("UNDO_UNINSTALL {}", op));
            } else {
                println!("Não sei desfazer automaticamente essa ação.");
            }
        }
    } else {
        println!("Nada para desfazer!");
    }
}

fn doctor() {
    println!("Checando SO e ferramentas...");
    println!("OS: {}", std::env::consts::OS);
    println!("Arch: {}", std::env::consts::ARCH);
    let has_brew = Command::new("sh")
        .arg("-c")
        .arg("which brew")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    println!("brew presente: {}", has_brew);
}

fn search_packages(cfg: &Config, query: &str) {
    println!("Procurando por {}...", query);
    let mut found = false;
    for pkg in &cfg.packages {
        if pkg.name.to_lowercase().contains(&query.to_lowercase()) {
            println!(
                "- {} (Linux: {:?}, Mac: {:?}, Win: {:?})",
                pkg.name, pkg.linux, pkg.mac, pkg.windows
            );
            found = true;
        }
    }
    if !found {
        println!("Nenhum pacote encontrado pra '{}'", query);
    }
}

fn add_package(mut cfg: Config, name: String, linux: String, mac: String, windows: String) {
    println!("Adicionando pacote custom: {}", name);
    let entry = PackageEntry {
        name: name.clone(),
        linux: Some(linux),
        mac: Some(mac),
        windows: Some(windows),
    };

    cfg.packages.push(entry);

    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        if let Err(e) = fs::write(config_path(), json) {
            println!("Erro ao salvar config: {}", e);
        } else {
            println!("Pacote '{}' adicionado com sucesso!", name);
        }
    }
}

fn check_alias() {
    if let Some(arg0) = env::args().next() {
        let exe_name = arg0.split('/').last().unwrap_or("");
        let aliases = [
            "minibrew", "mb", "mB", "MB", "Mb", "MiniBrew", "MinBrew", "miniBrew", "minBrew",
        ];
        if aliases.contains(&exe_name) {
            println!("Executando com '{}'", exe_name)
        } else {
            println!(
                "Aviso: executando via '{}', não e um alias registrado",
                exe_name
            );
        }
    }
}

fn show_info(cfg: &Config, package: &str) {
    if let Some(pkg) = cfg.packages.iter().find(|p| p.name == package) {
        println!("Info par '{}':", package);
        println!("  Linux:   {:?}", pkg.linux);
        println!("  MacOS:   {:?}", pkg.mac);
        println!(" Windws:   {:?}", pkg.windows);
    } else {
        println!("Pacote '{}' não encontrado no config.", package)
    }
}

fn list_packages(cfg: &Config) {
    println!("Pacotes disponiveis:");
    for pkg in &cfg.packages {
        println!("- {}", pkg.name);
    }
}

fn export_config() {
    let p = config_path();
    println!("Exportando config de {}", p.display());
    if let Ok(s) = fs::read_to_string(p) {
        let out = "minibrew-export.json";
        if fs::write(out, s).is_ok() {
            println!("Config exportado para '{}'", out);
        }
    }
}

fn import_config(file: &str) {
    println!("Importando config de {}", file);
    if let Ok(s) = fs::read_to_string(file) {
        if fs::write(config_path(), s).is_ok() {
            println!("Config importado com sucesso!");
        }
    }
}

fn needs_update() -> bool {
    if let Some(line) = last_action() {
        if line.contains("UPGRADE_ALL") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Ok(ts) = parts[0].parse::<u64>() {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let diff_days = (now - ts) / 86400;
                return diff_days >= 3;
            }
        }
    }
    true
}

fn main() {
    check_alias();
    ascii_banner();
    let cli = Cli::parse();
    let cfg = read_or_create_config();

    if needs_update() {
        println!(
            "AVISO: faz mais de 3 dias desde o último upgrade. Considere rodar 'minibrew upgrade-all'"
        );
    }

    match cli.cmd {
        Commands::Install { package } => {
            if package == "furry" {
                println!("EU ME RECUSO A BAIXA ESTA MERDA!!");
            } else if package == "java" {
                println!("Use Python seu ANIMAL!!");
            } else {
                install_package(&cfg, &package);
            }
        }
        Commands::Uninstall { package } => uninstall_package(&cfg, &package),
        Commands::Update { package } => update_package(&cfg, &package),
        Commands::UpgradeAll => upgrade_all(&cfg),
        Commands::Stack { stack } => install_stack(&cfg, &stack),
        Commands::SelfUpdate => self_update(),
        Commands::History => show_history(),
        Commands::Undo => undo_last(),
        Commands::Config => {
            println!("Abrindo arquivo de config: {}", config_path().display());
            if let Err(e) = open::that(config_path()) {
                println!("Não foi possivel abrir o arquivo de config: {}", e);
            }
        }
        Commands::Version => println!("MiniBrew v0.3"),
        Commands::Doctor => doctor(),
        Commands::Search { query } => search_packages(&cfg, &query),
        Commands::Add {
            name,
            linux,
            mac,
            windows,
        } => add_package(cfg, name, linux, mac, windows),
        Commands::Info { package } => show_info(&cfg, &package),
        Commands::List => list_packages(&cfg),
        Commands::Export => export_config(),
        Commands::Import { file } => import_config(&file),
    }
}
