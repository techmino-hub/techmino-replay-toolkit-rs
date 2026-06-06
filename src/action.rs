//! Module about actions in a single [`GameInputEvent`][crate::GameInputEvent].
//!
//! The action is stored as a single packed byte containing information on its kind
//! (press or release) as well as its key (e.g., move left, move right).

use core::num::TryFromIntError;
use serde::{Deserialize, Serialize};

/// Represents an action associated with a certain input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputAction {
    /// Whether or not this is a press action or a release action.
    pub kind: InputActionKind,
    /// Represents the key/button that was acted upon at this time.
    pub key: InputActionKey,
}

impl InputAction {
    /// Tries to convert an encoded byte into an [`InputAction`].
    ///
    /// This is the `const fn` version of the `TryFrom<u8>` implementation. For more information, see [`TryFrom`].
    ///
    /// # Errors
    /// This function errors if the given byte is not a valid encoded input action byte.
    pub const fn try_from_byte(value: u8) -> Result<Self, <InputActionKey as TryFrom<u8>>::Error> {
        let kind = if value >= 0b0010_0000 {
            InputActionKind::Release
        } else {
            InputActionKind::Press
        };

        let keycode = value & 0b001_1111;
        let key = match InputActionKey::try_from_byte(keycode) {
            Ok(k) => k,
            Err(e) => return Err(e),
        };

        Ok(Self { kind, key })
    }

    /// Converts this input action to its encoded byte.
    ///
    /// This is the `const fn` version of the `Into<u8>` implementation. For more information, see [`Into`].
    #[must_use]
    pub const fn into_byte(self) -> u8 {
        let kind_bits = match self.kind {
            InputActionKind::Release => 0b0010_0000,
            InputActionKind::Press => 0b0000_0000,
        };

        let key_bits = self.key.into_byte();

        key_bits | kind_bits
    }
}

impl TryFrom<u8> for InputAction {
    // TODO: Replace with actual error type
    type Error = <InputActionKey as TryFrom<u8>>::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        InputAction::try_from_byte(value)
    }
}

impl From<InputAction> for u8 {
    fn from(value: InputAction) -> Self {
        value.into_byte()
    }
}

/// Whether this is a press action or a release action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum InputActionKind {
    /// A certain button is being pressed in the event.
    Press = 0,
    /// A certain button is being released in the event.
    Release = 1,
}

impl InputActionKind {
    /// Converts a bool into the corresponding [`InputActionKind`] as used by the game.
    ///
    /// This is a `const fn` version of the corresponding `From<u8>` implementation. For more
    /// information, see [`From`].
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::Release
        } else {
            Self::Press
        }
    }

    /// Converts this
    #[must_use]
    pub const fn into_bool(self) -> bool {
        match self {
            Self::Release => true,
            Self::Press => false,
        }
    }
}

impl TryFrom<u8> for InputActionKind {
    type Error = TryFromIntError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_bool(bool::try_from(value)?))
    }
}

impl From<bool> for InputActionKind {
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

impl From<InputActionKind> for u8 {
    fn from(value: InputActionKind) -> Self {
        value.into_bool().into()
    }
}

impl From<InputActionKind> for bool {
    fn from(value: InputActionKind) -> Self {
        value.into_bool()
    }
}

/// Represents the key/button that was acted upon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "strum", derive(strum::EnumIter))]
#[allow(missing_docs)]
#[repr(u8)]
pub enum InputActionKey {
    MoveLeft = 1,
    MoveRight = 2,
    RotateRight = 3,
    RotateLeft = 4,
    Rotate180 = 5,
    HardDrop = 6,
    SoftDrop = 7,
    Hold = 8,

    Function1 = 9,
    Function2 = 10,

    InstantLeft = 11,
    InstantRight = 12,
    SonicDrop = 13,
    Down1 = 14,
    Down4 = 15,
    Down10 = 16,
    LeftDrop = 17,
    RightDrop = 18,
    LeftZangi = 19,
    RightZangi = 20,
}

impl InputActionKey {
    /// Tries to convert an encoded byte into an [`InputActionKey`]
    ///
    /// This is the `const fn` version of the `TryFrom<u8>` implementation. For more information, see [`TryFrom`].
    ///
    /// # Errors
    /// This function errors if the given byte is not a valid encoded input action byte.
    pub const fn try_from_byte(value: u8) -> Result<Self, ()> {
        use InputActionKey::{
            Down1, Down10, Down4, Function1, Function2, HardDrop, Hold, InstantLeft, InstantRight,
            LeftDrop, LeftZangi, MoveLeft, MoveRight, RightDrop, RightZangi, Rotate180, RotateLeft,
            RotateRight, SoftDrop, SonicDrop,
        };

        match value {
            1 => Ok(MoveLeft),
            2 => Ok(MoveRight),
            3 => Ok(RotateRight),
            4 => Ok(RotateLeft),
            5 => Ok(Rotate180),
            6 => Ok(HardDrop),
            7 => Ok(SoftDrop),
            8 => Ok(Hold),
            9 => Ok(Function1),
            10 => Ok(Function2),
            11 => Ok(InstantLeft),
            12 => Ok(InstantRight),
            13 => Ok(SonicDrop),
            14 => Ok(Down1),
            15 => Ok(Down4),
            16 => Ok(Down10),
            17 => Ok(LeftDrop),
            18 => Ok(RightDrop),
            19 => Ok(LeftZangi),
            20 => Ok(RightZangi),
            _ => Err(()),
        }
    }

    /// Converts an [`InputActionKey`] to its encoded byte representation.
    ///
    /// This is the `const fn` version of the corresponding `Into<u8>` implementation. For more
    /// information, see [`Into`].
    #[must_use]
    pub const fn into_byte(self) -> u8 {
        use InputActionKey::{
            Down1, Down10, Down4, Function1, Function2, HardDrop, Hold, InstantLeft, InstantRight,
            LeftDrop, LeftZangi, MoveLeft, MoveRight, RightDrop, RightZangi, Rotate180, RotateLeft,
            RotateRight, SoftDrop, SonicDrop,
        };

        match self {
            MoveLeft => 1,
            MoveRight => 2,
            RotateRight => 3,
            RotateLeft => 4,
            Rotate180 => 5,
            HardDrop => 6,
            SoftDrop => 7,
            Hold => 8,
            Function1 => 9,
            Function2 => 10,
            InstantLeft => 11,
            InstantRight => 12,
            SonicDrop => 13,
            Down1 => 14,
            Down4 => 15,
            Down10 => 16,
            LeftDrop => 17,
            RightDrop => 18,
            LeftZangi => 19,
            RightZangi => 20,
        }
    }
}

impl TryFrom<u8> for InputActionKey {
    // TODO: Replace with actual error type
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_from_byte(value)
    }
}

impl From<InputActionKey> for u8 {
    fn from(value: InputActionKey) -> Self {
        value.into_byte()
    }
}
