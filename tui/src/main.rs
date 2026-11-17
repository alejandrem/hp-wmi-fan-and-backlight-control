use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

const HWMON_BASE: &str = "/sys/devices/platform/hp-wmi/hwmon/";

fn find_hwmon() -> PathBuf {
    let base = std::path::Path::new(HWMON_BASE);
    let mut entries: Vec<_> = fs::read_dir(base)
        .unwrap_or_else(|e| {
            eprintln!("Error: no se encontro el modulo hp-wmi ({e})");
            process::exit(1);
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("hwmon") && e.path().is_dir()
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());
    entries
        .into_iter()
        .next()
        .map(|e| e.path())
        .unwrap_or_else(|| {
            eprintln!("Error: no se encontro ningun nodo hwmon");
            process::exit(1);
        })
}

fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| {
            eprintln!("Error al leer {:?}: {e}", path);
            process::exit(1);
        })
        .trim()
        .to_string()
}

fn write_file(path: &PathBuf, value: &str) {
    fs::write(path, value)
        .unwrap_or_else(|e| {
            eprintln!("Error al escribir {:?}: {e}", path);
            process::exit(1);
        });
}

fn cmd_status(hwmon: &PathBuf) {
    let mode = read_file(&hwmon.join("pwm1_enable"));
    let mode_text = match mode.as_str() {
        "0" => "MAX",
        "1" => "MANUAL",
        "2" => "AUTO",
        _ => "Desconocido",
    };

    let fan1 = read_file(&hwmon.join("fan1_input"));
    let fan2 = read_file(&hwmon.join("fan2_input"));

    println!("Modo: {mode_text}");
    println!("Fan1: {fan1} RPM");
    println!("Fan2: {fan2} RPM");

    if hwmon.join("fan1_target").exists() {
        let t1 = read_file(&hwmon.join("fan1_target"));
        let t2 = read_file(&hwmon.join("fan2_target"));
        let m1 = read_file(&hwmon.join("fan1_max"));
        let m2 = read_file(&hwmon.join("fan2_max"));
        println!("Fan1 target: {t1} RPM");
        println!("Fan2 target: {t2} RPM");
        println!("Fan1 max (BIOS): {m1} RPM");
        println!("Fan2 max (BIOS): {m2} RPM");
    }
}

fn cmd_rpm(hwmon: &PathBuf, rpm_str: &str) {
    let rpm: i64 = rpm_str.parse().unwrap_or_else(|_| {
        eprintln!("Error: '{rpm_str}' no es un numero valido");
        process::exit(1);
    });

    if rpm < 0 || rpm > 6500 {
        eprintln!("Error: RPM debe estar entre 0 y 6500");
        process::exit(1);
    }

    let target1 = hwmon.join("fan1_target");
    if !target1.exists() {
        eprintln!("Error: fan_target no disponible en este driver");
        process::exit(1);
    }

    write_file(&hwmon.join("pwm1_enable"), "1");
    write_file(&hwmon.join("fan1_target"), &rpm.to_string());
    write_file(&hwmon.join("fan2_target"), &rpm.to_string());

    println!("RPM objetivo: {rpm} (esperando a que el hardware responda...)");
    std::thread::sleep(std::time::Duration::from_secs(10));

    let fan1 = read_file(&hwmon.join("fan1_input"));
    let fan2 = read_file(&hwmon.join("fan2_input"));
    println!("Fan1: {fan1} RPM");
    println!("Fan2: {fan2} RPM");
}

fn cmd_auto(hwmon: &PathBuf) {
    write_file(&hwmon.join("pwm1_enable"), "2");
    println!("Modo AUTO activado (control del BIOS)");
}

fn cmd_max(hwmon: &PathBuf) {
    write_file(&hwmon.join("pwm1_enable"), "0");
    println!("Modo MAX activado (100% fijo)");
}

fn print_usage() {
    println!("Control de ventiladores HP Victus");
    println!();
    println!("Uso:");
    println!("  fanctl rpm [0-6500]  - Velocidad RPM exacta (activa modo MANUAL)");
    println!("  fanctl auto          - Control automatico del BIOS");
    println!("  fanctl max           - Ventiladores al maximo fijo");
    println!("  fanctl status        - Ver estado actual");
    println!();
    println!("Ejemplos:");
    println!("  fanctl rpm 3000      - 3000 RPM exactos");
    println!("  fanctl rpm 5000      - 5000 RPM (por encima del limite BIOS)");
    println!("  fanctl rpm 6000      - Maximo hardware (~6000 RPM)");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let hwmon = find_hwmon();

    match args.get(1).map(|s| s.as_str()) {
        Some("status") => cmd_status(&hwmon),
        Some("rpm") => {
            let rpm = args.get(2).unwrap_or_else(|| {
                eprintln!("Uso: fanctl rpm [0-6500]");
                process::exit(1);
            });
            cmd_rpm(&hwmon, rpm);
        }
        Some("auto") => cmd_auto(&hwmon),
        Some("max") => cmd_max(&hwmon),
        Some(_) => {
            print_usage();
            process::exit(1);
        }
        None => print_usage(),
    }
}
