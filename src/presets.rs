pub struct GamePresets;

#[derive(Clone)]
pub struct GamePreset {
    pub name: &'static str,
    pub dxvk: bool,
    pub win32: bool,
    pub deps: &'static [&'static str],
    pub dll_overrides: &'static str,
    pub env_vars: &'static [(&'static str, &'static str)],
    pub notes: &'static str,
    pub suggested_proton: &'static str,
}

impl GamePresets {
    pub fn get_preset(game_name: &str) -> Option<GamePreset> {
        let name_lower = game_name.to_lowercase();

        const PRESETS: &[(&str, GamePreset)] = &[
            (
                "city car driving",
                GamePreset {
                    name: "City Car Driving",
                    dxvk: false,
                    win32: true,
                    deps: &[
                        "d3dx9",
                        "vcrun2005",
                        "vcrun2008",
                        "dotnet40",
                        "physx",
                        "xact",
                        "d3dcompiler_43",
                    ],
                    dll_overrides: "ole32=b;oleaut32=b;d3d9=b,n;d3d11=b,n;dxgi=b,n;xaudio2_7=n,b",
                    env_vars: &[
                        ("WINE_LARGE_ADDRESS_AWARE", "1"),
                        ("WINE_CPU_TOPOLOGY", "1"),
                    ],
                    notes: "Needs .NET, PhysX and 32-bit prefix. Use WineD3D.",
                    suggested_proton: "System Wine (Default)",
                },
            ),
            (
                "need for speed",
                GamePreset {
                    name: "Need for Speed Series",
                    dxvk: true,
                    win32: false,
                    deps: &["vcrun2015", "d3dx9", "physx"],
                    dll_overrides: "d3d9=d,d3d11=d,dxgi=d",
                    env_vars: &[],
                    notes: "Most NFS games work with DXVK",
                    suggested_proton: "GE-Proton",
                },
            ),
            (
                "cyberpunk",
                GamePreset {
                    name: "Cyberpunk 2077",
                    dxvk: true,
                    win32: false,
                    deps: &["vcrun2015", "vcrun2017", "vcrun2019", "vcrun2022"],
                    dll_overrides: "d3d9=d,d3d11=d,dxgi=d",
                    env_vars: &[],
                    notes: "Use latest DXVK and Proton GE",
                    suggested_proton: "GE-Proton",
                },
            ),
            (
                "stalker",
                GamePreset {
                    name: "S.T.A.L.K.E.R.",
                    dxvk: true,
                    win32: true,
                    deps: &["d3dx9", "d3dcompiler_43", "vcrun2005", "physx"],
                    dll_overrides: "d3d9=d,d3d11=d",
                    env_vars: &[("WINE_LARGE_ADDRESS_AWARE", "1")],
                    notes: "",
                    suggested_proton: "GE-Proton",
                },
            ),
            (
                "gta vice city",
                GamePreset {
                    name: "GTA Vice City",
                    dxvk: false,
                    win32: true,
                    deps: &["vcrun2005", "d3dx9"],
                    dll_overrides: "d3d9=b,n,d3d8=b",
                    env_vars: &[("WINE_LARGE_ADDRESS_AWARE", "1")],
                    notes: "Classic GTA",
                    suggested_proton: "System Wine (Default)",
                },
            ),
            (
                "sims 3",
                GamePreset {
                    name: "The Sims 3",
                    dxvk: false,
                    win32: true,
                    deps: &["d3dx9", "vcrun2005", "dotnet40"],
                    dll_overrides: "d3d9=b,n",
                    env_vars: &[("WINE_LARGE_ADDRESS_AWARE", "1")],
                    notes: "",
                    suggested_proton: "System Wine (Default)",
                },
            ),
        ];

        for (key, preset) in PRESETS {
            if name_lower.contains(key) {
                return Some((*preset).clone());
            }
        }

        None
    }

    #[allow(dead_code)]
    pub fn is_legacy_game(game_name: &str) -> bool {
        let legacy_indicators = [
            "city car",
            "nfs",
            "need for speed",
            "gta vice",
            "gta san",
            "sims 1",
            "sims 2",
            "sims 3",
            "stalker",
            "half-life",
            "counter-strike",
            "warcraft",
            "starcraft",
        ];

        let name_lower = game_name.to_lowercase();
        legacy_indicators.iter().any(|x| name_lower.contains(x))
    }

    #[allow(dead_code)]
    pub fn get_all_preset_names() -> Vec<&'static str> {
        vec![
            "City Car Driving",
            "Need for Speed",
            "GTA Vice City",
            "S.T.A.L.K.E.R.",
            "The Sims 3",
            "Cyberpunk 2077",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset_existing() {
        let preset = GamePresets::get_preset("GTA Vice City").unwrap();
        assert_eq!(preset.name, "GTA Vice City");
        assert_eq!(preset.win32, true);
    }

    #[test]
    fn test_get_preset_lowercase() {
        let preset = GamePresets::get_preset("cyberpunk 2077").unwrap();
        assert_eq!(preset.name, "Cyberpunk 2077");
        assert_eq!(preset.dxvk, true);
    }

    #[test]
    fn test_is_legacy_game() {
        assert!(GamePresets::is_legacy_game("City Car Driving"));
        assert!(GamePresets::is_legacy_game("nfs most wanted"));
        assert!(!GamePresets::is_legacy_game("Cyberpunk 2077"));
    }
}
