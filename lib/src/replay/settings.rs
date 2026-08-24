//! Represents stuff found in game settings.

/// Represents the skin color of a piece in the game.
///
/// The discriminants are the numbers used by the game to identify each color
/// in the replay metadata.
///
/// The approximate hue may differ between piece skin sets. The given numbers
/// are loosely derived from the `pure` skin by `MrZ`.
#[repr(u8)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "strum",
    derive(strum::EnumIter, strum::IntoStaticStr, strum::VariantArray)
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PieceColor {
    /// Red, the R in RGB.
    ///
    /// Hue ≈ 0°
    Red = 1,
    /// A warm color in between red and orange.
    ///
    /// Hue ≈ 25°
    #[doc(alias = "OrangeRed")]
    Fire = 2,
    /// A warm color in between red and yellow.
    ///
    /// Hue ≈ 40°
    Orange = 3,
    /// A warm color in between red and green.
    ///
    /// Hue ≈ 60°
    Yellow = 4,
    /// A mixture of yellow and green, closer to yellow.
    ///
    /// Hue ≈ 80°
    Lime = 5,
    /// A mixture of yellow and green, closer to green.
    ///
    /// Hue ≈ 100°
    Jade = 6,
    /// Green, the G in RGB.
    ///
    /// Hue ≈ 120°
    Green = 7,
    /// A cool color in between green and cyan.
    ///
    /// Hue ≈ 150°
    Aqua = 8,
    /// A cool color in between green and blue.
    ///
    /// Hue ≈ 180°
    Cyan = 9,
    /// A cool color in between cyan and blue, closer to cyan.
    ///
    /// Hue ≈ 200°
    Navy = 10,
    /// A cool color in between cyan and blue, closer to blue.
    ///
    /// Hue ≈ 220°
    Sea = 11,
    /// Blue, the B in RGB.
    ///
    /// Hue ≈ 240°
    Blue = 12,
    /// A cool color in between blue and purple.
    ///
    /// Hue ≈ 260°
    Violet = 13,
    /// A cool color, and a mix of blue and red, closer to blue.
    ///
    /// Hue ≈ 280°
    Purple = 14,
    /// A vibrant color which is a mix of blue and red.
    ///
    /// Hue ≈ 310°
    Magenta = 15,
    /// A vibrant color which is a mix of blue and red, closer to red.
    ///
    /// Hue ≈ 340°
    Wine = 16,
    /// The bone skin variant, commonly seen in high-level 20G runs.
    ///
    /// No common color.
    Bone = 17,
    /// The invisible skin variant.
    #[doc(alias = "None")]
    Invisible = 18,
    /// The bomb skin variant, with special behaviour.
    ///
    /// Usually only seen in the Custom game mode, where
    /// if a piece is placed on top of a mino in this color, the entire line
    /// containing this mino is removed.
    Bomb = 19,
    /// The dark grey (dark gray) skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    #[doc(alias = "DarkGray")]
    DarkGrey = 20,
    /// The light grey (light gray) skin color.
    ///
    /// (Light as in the color of the garbage, not the amount.)
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    #[doc(alias = "LightGray")]
    LightGrey = 21,
    /// The light violet skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    ///
    /// Hue ≈ 245°
    LightViolet = 22,
    /// The light magenta skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    ///
    /// Hue ≈ 315°
    LightMagenta = 23,
    /// The light green skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    ///
    /// Hue ≈ 145°
    LightGreen = 24,
}

impl PieceColor {
    /// Converts this [`PieceColor`] into its `u8` representation.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Tries to convert a `u8` into a [`PieceColor`] if it is valid.
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Red),
            2 => Some(Self::Fire),
            3 => Some(Self::Orange),
            4 => Some(Self::Yellow),
            5 => Some(Self::Lime),
            6 => Some(Self::Jade),
            7 => Some(Self::Green),
            8 => Some(Self::Aqua),
            9 => Some(Self::Cyan),
            10 => Some(Self::Navy),
            11 => Some(Self::Sea),
            12 => Some(Self::Blue),
            13 => Some(Self::Violet),
            14 => Some(Self::Purple),
            15 => Some(Self::Magenta),
            16 => Some(Self::Wine),
            17 => Some(Self::Bone),
            18 => Some(Self::Invisible),
            19 => Some(Self::Bomb),
            20 => Some(Self::DarkGrey),
            21 => Some(Self::LightGrey),
            22 => Some(Self::LightViolet),
            23 => Some(Self::LightMagenta),
            24 => Some(Self::LightGreen),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_color_enum_u8_roundtrip() {
        for i in 0..u8::MAX {
            let Some(color) = PieceColor::try_from_u8(i) else {
                continue;
            };

            assert_eq!(color.as_u8(), i);
        }
    }
}
