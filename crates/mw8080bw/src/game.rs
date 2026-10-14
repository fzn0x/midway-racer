//! The games that run on the racing variant of the board.

/// A game supported by the emulator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Game {
    /// *Laguna Racer* (Midway, 1977).
    LagunaRacer,
    /// *280-ZZZAP* (Dave Nutting Associates / Midway, 1976), on six 1 KiB
    /// ROMs.
    Zzzap,
    /// *280-ZZZAP*, alternative set on three 2 KiB ROMs.
    ZzzapSet2,
}

impl Game {
    /// Every supported game, Laguna Racer first.
    pub const ALL: [Self; 3] = [Self::LagunaRacer, Self::Zzzap, Self::ZzzapSet2];

    /// Returns the static description of the game.
    pub const fn info(self) -> &'static GameInfo {
        match self {
            Self::LagunaRacer => &LAGUNA_RACER,
            Self::Zzzap => &ZZZAP,
            Self::ZzzapSet2 => &ZZZAP_SET_2,
        }
    }

    /// Looks a game up by its MAME short name, such as `lagunar`.
    pub fn from_short_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|game| game.info().short_name.eq_ignore_ascii_case(name))
    }
}

/// Static facts about a game.
#[derive(Debug)]
pub struct GameInfo {
    /// MAME's short name for the set, which is also its archive name.
    pub short_name: &'static str,
    /// The title shown to players.
    pub title: &'static str,
    /// Year of release.
    pub year: u16,
    /// Who made it.
    pub manufacturer: &'static str,
    /// The program ROM chips, in address order.
    pub roms: &'static [RomChip],
    /// The DIP switch bank read on input port 2.
    pub dip_switches: &'static [DipSwitch],
    /// The factory setting of the DIP switch bank.
    pub default_dip_switches: u8,
    /// How the monitor is mounted in the cabinet.
    pub orientation: Orientation,
    /// Coloured bands of the overlay film on the monitor glass. Areas
    /// outside every band show the phosphor's white.
    pub overlay: &'static [OverlayBand],
}

/// One chip of a program ROM set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RomChip {
    /// The file name MAME uses for the chip's dump.
    pub name: &'static str,
    /// Where the chip appears in the CPU's address space.
    pub offset: usize,
    /// The chip's size in bytes.
    pub size: usize,
    /// CRC-32 of a good dump.
    pub crc32: u32,
}

/// How the monitor is mounted in the cabinet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Mounted normally, showing a landscape picture (MAME's `ROT0`).
    Horizontal,
    /// Mounted on its side, rotated 90° clockwise, showing a portrait
    /// picture (MAME's `ROT90`).
    Vertical,
}

/// A group of DIP switches that together select one setting.
#[derive(Debug)]
pub struct DipSwitch {
    /// What the switches control.
    pub name: &'static str,
    /// The bits of the switch bank the group occupies.
    pub mask: u8,
    /// The possible settings.
    pub settings: &'static [DipSetting],
}

impl DipSwitch {
    /// Returns the setting selected by the switch bank value `dips`.
    pub fn setting(&self, dips: u8) -> Option<&DipSetting> {
        self.settings
            .iter()
            .find(|setting| setting.value == dips & self.mask)
    }
}

/// One setting of a [`DipSwitch`].
#[derive(Debug)]
pub struct DipSetting {
    /// The value of the switch bits for this setting.
    pub value: u8,
    /// A description of the setting.
    pub label: &'static str,
}

/// A colour, in 8-bit sRGB components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// The red component.
    pub red: u8,
    /// The green component.
    pub green: u8,
    /// The blue component.
    pub blue: u8,
}

impl Rgb {
    /// Pure white, the colour of the monitor's phosphor.
    pub const WHITE: Self = Self::new(0xff, 0xff, 0xff);
    /// Black, the colour of an unlit screen.
    pub const BLACK: Self = Self::new(0x00, 0x00, 0x00);

    /// Creates a colour from its components.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

/// A horizontal band of tinted overlay film, in unrotated screen lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayBand {
    /// First screen line covered by the band.
    pub top: usize,
    /// First screen line below the band.
    pub bottom: usize,
    /// The colour the band tints lit pixels.
    pub color: Rgb,
}

const COINAGE: DipSwitch = DipSwitch {
    name: "Coinage",
    mask: 0x03,
    settings: &[
        DipSetting {
            value: 0x02,
            label: "2 Coins / 1 Credit",
        },
        DipSetting {
            value: 0x00,
            label: "1 Coin / 1 Credit",
        },
        DipSetting {
            value: 0x03,
            label: "2 Coins / 3 Credits",
        },
        DipSetting {
            value: 0x01,
            label: "1 Coin / 2 Credits",
        },
    ],
};

static LAGUNA_RACER: GameInfo = GameInfo {
    short_name: "lagunar",
    title: "Laguna Racer",
    year: 1977,
    manufacturer: "Midway",
    roms: &[
        RomChip {
            name: "lagunar.h",
            offset: 0x0000,
            size: 0x0800,
            crc32: 0x0cd5_a280,
        },
        RomChip {
            name: "lagunar.g",
            offset: 0x0800,
            size: 0x0800,
            crc32: 0x824c_d6f5,
        },
        RomChip {
            name: "lagunar.f",
            offset: 0x1000,
            size: 0x0800,
            crc32: 0x6269_2ca7,
        },
        RomChip {
            name: "lagunar.e",
            offset: 0x1800,
            size: 0x0800,
            crc32: 0x20e0_98ed,
        },
    ],
    dip_switches: &[
        COINAGE,
        DipSwitch {
            name: "Game Time",
            mask: 0x0c,
            settings: &[
                DipSetting {
                    value: 0x00,
                    label: "45 seconds + 22 extended",
                },
                DipSetting {
                    value: 0x04,
                    label: "60 seconds + 30 extended",
                },
                DipSetting {
                    value: 0x08,
                    label: "75 seconds + 37 extended",
                },
                DipSetting {
                    value: 0x0c,
                    label: "90 seconds + 45 extended",
                },
            ],
        },
        DipSwitch {
            name: "Extended Time At",
            mask: 0x30,
            settings: &[
                DipSetting {
                    value: 0x00,
                    label: "350",
                },
                DipSetting {
                    value: 0x10,
                    label: "400",
                },
                DipSetting {
                    value: 0x20,
                    label: "450",
                },
                DipSetting {
                    value: 0x30,
                    label: "500",
                },
            ],
        },
        DipSwitch {
            name: "Test Modes / Extended Time",
            mask: 0xc0,
            settings: &[
                DipSetting {
                    value: 0x00,
                    label: "Off",
                },
                DipSetting {
                    value: 0x40,
                    label: "RAM/ROM Test",
                },
                DipSetting {
                    value: 0x80,
                    label: "Input Test",
                },
                DipSetting {
                    value: 0xc0,
                    label: "No Extended Time",
                },
            ],
        },
    ],
    default_dip_switches: 0x24,
    orientation: Orientation::Vertical,
    overlay: &[],
};

const ZZZAP_DIP_SWITCHES: &[DipSwitch] = &[
    COINAGE,
    DipSwitch {
        name: "Game Time",
        mask: 0x0c,
        settings: &[
            DipSetting {
                value: 0x0c,
                label: "60 seconds + 30 extended",
            },
            DipSetting {
                value: 0x00,
                label: "80 seconds + 40 extended",
            },
            DipSetting {
                value: 0x08,
                label: "99 seconds + 50 extended",
            },
            DipSetting {
                value: 0x04,
                label: "Test Mode",
            },
        ],
    },
    DipSwitch {
        name: "Extended Time At",
        mask: 0x30,
        settings: &[
            DipSetting {
                value: 0x10,
                label: "2.00",
            },
            DipSetting {
                value: 0x00,
                label: "2.50",
            },
            DipSetting {
                value: 0x20,
                label: "None",
            },
        ],
    },
    DipSwitch {
        name: "Language",
        mask: 0xc0,
        settings: &[
            DipSetting {
                value: 0x00,
                label: "English",
            },
            DipSetting {
                value: 0x40,
                label: "German",
            },
            DipSetting {
                value: 0x80,
                label: "French",
            },
            DipSetting {
                value: 0xc0,
                label: "Spanish",
            },
        ],
    },
];

/// The blue film over the bottom of the 280-ZZZAP screen.
const ZZZAP_OVERLAY: &[OverlayBand] = &[OverlayBand {
    top: 181,
    bottom: 224,
    color: Rgb::new(0x33, 0x66, 0xff),
}];

static ZZZAP: GameInfo = GameInfo {
    short_name: "280zzzap",
    title: "280-ZZZAP",
    year: 1976,
    manufacturer: "Dave Nutting Associates / Midway",
    roms: &[
        RomChip {
            name: "zzzaph",
            offset: 0x0000,
            size: 0x0400,
            crc32: 0x1fa8_6e1c,
        },
        RomChip {
            name: "zzzapg",
            offset: 0x0400,
            size: 0x0400,
            crc32: 0x9639_bc6b,
        },
        RomChip {
            name: "zzzapf",
            offset: 0x0800,
            size: 0x0400,
            crc32: 0xadc6_ede1,
        },
        RomChip {
            name: "zzzape",
            offset: 0x0c00,
            size: 0x0400,
            crc32: 0x4724_93d6,
        },
        RomChip {
            name: "zzzapd",
            offset: 0x1000,
            size: 0x0400,
            crc32: 0x4c24_0ee1,
        },
        RomChip {
            name: "zzzapc",
            offset: 0x1400,
            size: 0x0400,
            crc32: 0x6e85_aeaf,
        },
    ],
    dip_switches: ZZZAP_DIP_SWITCHES,
    default_dip_switches: 0x00,
    orientation: Orientation::Horizontal,
    overlay: ZZZAP_OVERLAY,
};

static ZZZAP_SET_2: GameInfo = GameInfo {
    short_name: "280zzzapa",
    title: "280-ZZZAP (set 2)",
    year: 1976,
    manufacturer: "Dave Nutting Associates / Midway",
    roms: &[
        RomChip {
            name: "h",
            offset: 0x0000,
            size: 0x0800,
            crc32: 0xcb56_e0f8,
        },
        RomChip {
            name: "g",
            offset: 0x0800,
            size: 0x0800,
            crc32: 0xf30d_d025,
        },
        RomChip {
            name: "f",
            offset: 0x1000,
            size: 0x0800,
            crc32: 0x8d51_ec2d,
        },
    ],
    dip_switches: ZZZAP_DIP_SWITCHES,
    default_dip_switches: 0x00,
    orientation: Orientation::Horizontal,
    overlay: ZZZAP_OVERLAY,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ROM_SIZE;

    #[test]
    fn short_names_are_unique_and_resolve() {
        for game in Game::ALL {
            let name = game.info().short_name;
            assert_eq!(Game::from_short_name(name), Some(game));
            assert_eq!(Game::from_short_name(&name.to_uppercase()), Some(game));
        }
        assert_eq!(Game::from_short_name("invaders"), None);
    }

    #[test]
    fn rom_chips_are_contiguous_and_fit_the_rom_area() {
        for game in Game::ALL {
            let mut next = 0;
            for chip in game.info().roms {
                assert_eq!(chip.offset, next, "{}", chip.name);
                next += chip.size;
            }
            assert!(next <= ROM_SIZE);
        }
    }

    #[test]
    fn dip_switches_cover_the_bank_and_default_to_valid_settings() {
        for game in Game::ALL {
            let info = game.info();
            let mut covered = 0;
            for switch in info.dip_switches {
                assert_eq!(covered & switch.mask, 0, "{} overlaps", switch.name);
                covered |= switch.mask;
                assert!(switch.setting(info.default_dip_switches).is_some());
                for setting in switch.settings {
                    assert_eq!(setting.value & !switch.mask, 0, "{}", setting.label);
                }
            }
            assert_eq!(covered, 0xff, "{}", info.title);
        }
    }

    #[test]
    fn laguna_racer_defaults_match_the_factory_settings() {
        let info = Game::LagunaRacer.info();
        let labels: Vec<_> = info
            .dip_switches
            .iter()
            .map(|switch| switch.setting(info.default_dip_switches).unwrap().label)
            .collect();
        assert_eq!(
            labels,
            [
                "1 Coin / 1 Credit",
                "60 seconds + 30 extended",
                "450",
                "Off"
            ]
        );
    }
}
