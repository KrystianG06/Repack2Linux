use std::process::Command;
use std::path::Path;
use std::fs;

fn main() {
    // Skrypt budujący - wersja "Safe-Mode" dla dysków HDD/NTFS
    let out_dir = ".tools";
    
    // Tworzymy folder tylko jeśli go nie ma
    if !Path::new(out_dir).exists() {
        let _ = fs::create_dir_all(out_dir);
    }

    let tools = ["wrestool", "icotool"];
    for tool in tools {
        let dest_path = format!("{}/{}", out_dir, tool);
        
        // Na dyskach NTFS kopiowanie binarek z /usr/bin może powodować Permission Denied
        // jeśli system plików nie obsługuje atrybutów wykonywania.
        // Dlatego najpierw sprawdzamy uprawnienia.
        if !Path::new(&dest_path).exists() {
            if let Ok(output) = Command::new("which").arg(tool).output() {
                if output.status.success() {
                    let tool_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    // Próbujemy skopiować, ale nie przejmujemy się jeśli zawiedzie przez system plików
                    if let Err(_) = fs::copy(&tool_path, &dest_path) {
                        println!("cargo:warning=Nie udało się skopiować {} do .tools (prawdopodobnie ograniczenie dysku HDD/NTFS). Używam wersji systemowej.", tool);
                    }
                }
            }
        }
    }

    // WAŻNE: Nie każemy Cargo skanować folderu .tools, jeśli to on sprawia problemy
    // println!("cargo:rerun-if-changed=.tools"); // Wyłączamy to tymczasowo
    println!("cargo:rerun-if-changed=build.rs");
}
