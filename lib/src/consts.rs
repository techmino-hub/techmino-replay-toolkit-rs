//! Module for any constants related to Techmino, especially its replays

use core::num::NonZeroU8;

use crate::replay::PieceColor;

/// The total amount of pieces in the current game.
///
/// There are currently 29 elements:
/// - 1 monomino
/// - 1 domino
/// - 2 trominoes
/// - 7 tetrominoes
/// - 18 pentominoes
pub const TOTAL_PIECE_COUNT: usize = 29;

/// Zlib always begins with 0x78 (`x`). \
/// <https://en.wikipedia.org/wiki/List_of_file_signatures>
pub const ZLIB_HEADER_FIRST_BYTE: u8 = b'x';
/// 0x7800 until 0x78FF always starts with an `e` in base64
pub const BASE64_ZLIB_FIRST_BYTE: u8 = b'e';
/// Raw uncompressed game data begins with a JSON object, which begins with a `{`
pub const UNCOMPRESSED_FIRST_BYTE: u8 = b'{';

/// The separator between the metadata and input event data sections of the raw
/// (uncompressed) versions of the replay.
pub const METADATA_EVENTDATA_SEPARATOR: u8 = b'\n';

/// The format string the game uses to format the date for the replay's metadata.
pub static METADATA_DATE_FORMAT: &str = "%Y/%m/%d %H:%M:%S";

/// The default skin color for each piece, represented as `u8`s.
///
/// For the [`PieceColor`] representation, see [`DEFAULT_SKIN_COLORS`].
///
/// # Piece-Specific Information
/// Use the [`Piece`] enum to help index into the given array.
///
/// ```
/// use libtechmino_replay::consts::{TOTAL_PIECE_COUNT, Piece, DEFAULT_SKIN_COLORS};
///
/// let t_skin = DEFAULT_SKIN_COLORS[Piece::T.get_index()];
/// ```
pub const DEFAULT_SKIN_COLORS_U8: [u8; TOTAL_PIECE_COUNT] = [
    1, 7, 11, 3, 14, 4, 9, 1, 7, 2, 6, 10, 2, 13, 5, 9, 15, 4, 11, 3, 12, 2, 16, 8, 4, 10, 13, 2, 8,
];

/// The default skin color for each piece, represented as [`PieceColor`]s.
///
/// For the `u8` representation, see [`DEFAULT_SKIN_COLORS_U8`].
///
/// # Piece-Specific Information
/// Use the [`Piece`] enum to help index into the given array.
///
/// ```
/// use libtechmino_replay::consts::{TOTAL_PIECE_COUNT, Piece, DEFAULT_SKIN_COLORS};
///
/// let t_skin = DEFAULT_SKIN_COLORS[Piece::T.get_index()];
/// ```
pub const DEFAULT_SKIN_COLORS: [PieceColor; TOTAL_PIECE_COUNT] = {
    let mut arr = [PieceColor::Invisible; TOTAL_PIECE_COUNT];
    let mut i = 0;
    while i < arr.len() {
        arr[i] = PieceColor::try_from_u8(DEFAULT_SKIN_COLORS_U8[i]).unwrap();
        i += 1;
    }

    arr
};

/// Represents a polyomino piece in the game.
///
/// Their determinants determine the index of the piece-related data.
///
/// # Example
/// ```
/// use libtechmino_replay::consts::{Piece, TOTAL_PIECE_COUNT};
///
/// # fn get_piece_data() -> [u8; TOTAL_PIECE_COUNT] { [0; TOTAL_PIECE_COUNT] }
/// # type PieceData = u8;
/// // type PieceData = ...;
///
/// let my_piece_data: [PieceData; TOTAL_PIECE_COUNT] = get_piece_data();
///
/// // Get the T-piece data
/// let t_piece_data: PieceData = my_piece_data[Piece::T.get_index()];
/// ```
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "strum",
    derive(strum::EnumIter, strum::IntoStaticStr, strum::VariantArray)
)]
#[cfg_attr(all(test, feature = "strum"), derive(strum::EnumCount))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum Piece {
    /// The Z-tetromino.
    ///
    /// ```text
    /// [][]
    ///   [][]
    /// ```
    Z = 0,
    /// The S-tetromino.
    ///
    /// ```text
    ///   [][]
    /// [][]
    /// ```
    S = 1,
    /// The J-tetromino.
    ///
    /// ```text
    /// []
    /// [][][]
    /// ```
    J = 2,
    /// The L-tetromino.
    ///
    /// ```text
    ///     []
    /// [][][]
    /// ```
    L = 3,
    /// The T-tetromino.
    ///
    /// ```text
    ///   []
    /// [][][]
    /// ```
    T = 4,
    /// The O-tetromino.
    ///
    /// ```text
    /// [][]
    /// [][]
    /// ```
    O = 5,
    /// The I-tetromino.
    ///
    /// ```text
    /// [][][][]
    /// ```
    I = 6,
    /// The Z5-pentomino.
    ///
    /// a.k.a.: the Z-pentomino
    ///
    /// ```text
    /// [][]
    ///   []
    ///   [][]
    /// ```
    Z5 = 7,
    /// The S5-pentomino.
    ///
    /// a.k.a.: the S-pentomino
    ///
    /// ```text
    ///   [][]
    ///   []
    /// [][]
    /// ```
    S5 = 8,
    /// The P-pentomino.
    ///
    /// ```text
    /// [][]
    /// [][][]
    /// ```
    P = 9,
    /// The Q-pentomino.
    ///
    /// ```text
    ///   [][]
    /// [][][]
    /// ```
    Q = 10,
    /// The F-pentomino.
    ///
    /// ```text
    /// []
    /// [][][]
    ///   []
    /// ```
    F = 11,
    /// The E-pentomino.
    ///
    /// a.k.a.: the F'-pentomino
    ///
    /// ```text
    ///     []
    /// [][][]
    ///   []
    /// ```
    E = 12,
    /// The T5-pentomino.
    ///
    /// a.k.a.: the T-pentomino
    ///
    /// ```text
    ///   []
    ///   []
    /// [][][]
    /// ```
    T5 = 13,
    /// The U-pentomino.
    ///
    /// ```text
    /// []  []
    /// [][][]
    /// ```
    U = 14,
    /// The V-pentomino.
    ///
    /// ```text
    ///     []
    ///     []
    /// [][][]
    /// ```
    V = 15,
    /// The W-pentomino.
    ///
    /// ```text
    /// []
    /// [][]
    ///   [][]
    /// ```
    W = 16,
    /// The X-pentomino.
    ///
    /// ```text
    ///   []
    /// [][][]
    ///   []
    /// ```
    X = 17,
    /// The J5-pentomino.
    ///
    /// a.k.a.: the J-pentomino
    ///
    /// ```text
    /// []
    /// [][][][]
    /// ```
    J5 = 18,
    /// The L5-pentomino.
    ///
    /// a.k.a.: The L-pentomino
    ///
    /// ```text
    ///       []
    /// [][][][]
    /// ```
    L5 = 19,
    /// The R-pentomino.
    ///
    /// a.k.a.: the Y'-pentomino
    ///
    /// ```text
    ///   []
    /// [][][][]
    /// ```
    R = 20,
    /// The Y-pentomino.
    ///
    /// ```text
    ///     []
    /// [][][][]
    /// ```
    Y = 21,
    /// The N-pentomino.
    ///
    /// ```text
    /// [][]
    ///   [][][]
    /// ```
    N = 22,
    /// The H-pentomino.
    ///
    /// a.k.a.: the N'-pentomino
    ///
    /// ```text
    ///     [][]
    /// [][][]
    /// ```
    H = 23,
    /// The I5-pentomino.
    ///
    /// a.k.a.: the I-pentomino
    ///
    /// ```text
    /// [][][][][]
    /// ```
    I5 = 24,
    /// The I3-triomino.
    ///
    /// a.k.a.: the I-tromino, the I-trimino, the I-triomino, the I3-tromino,
    /// the I3-trimino
    ///
    /// ```text
    /// [][][]
    /// ```
    I3 = 25,
    /// The C-triomino.
    ///
    /// a.k.a.: the L-tromino, the L-trimino, the L-triomino, the V-tromino,
    /// the V-trimino, the V-triomino, the C-tromino, the C-triomino
    ///
    /// ```text
    ///   []
    /// [][]
    /// ```
    C = 26,
    /// The I2-domino.
    ///
    /// a.k.a.: the I-domino, the domino
    ///
    /// ```text
    /// [][]
    /// ```
    I2 = 27,
    /// The O1-monomino.
    ///
    /// a.k.a.: the monomino
    ///
    /// ```text
    /// []
    /// ```
    O1 = 28,
}

impl Piece {
    /// Gets the index of an array that corresponds to this piece.
    #[must_use]
    pub const fn get_index(self) -> usize {
        self as usize
    }

    /// Get the amount of minos this piece has.
    ///
    /// Monominoes have one, dominoes have two, triominoes have three,
    /// tetrominoes have four, and pentominoes have five.
    #[must_use]
    pub const fn mino_count(self) -> NonZeroU8 {
        use Piece::{
            C, E, F, H, I, I2, I3, I5, J, J5, L, L5, N, O, O1, P, Q, R, S, S5, T, T5, U, V, W, X,
            Y, Z, Z5,
        };

        const ONE: NonZeroU8 = NonZeroU8::MIN;
        const TWO: NonZeroU8 = NonZeroU8::new(2).unwrap();
        const THREE: NonZeroU8 = NonZeroU8::new(3).unwrap();
        const FOUR: NonZeroU8 = NonZeroU8::new(4).unwrap();
        const FIVE: NonZeroU8 = NonZeroU8::new(5).unwrap();

        match self {
            O1 => ONE,
            I2 => TWO,
            C | I3 => THREE,
            Z | S | J | L | T | O | I => FOUR,
            Z5 | S5 | P | Q | F | E | T5 | U | V | W | X | J5 | L5 | R | Y | N | H | I5 => FIVE,
        }
    }

    /// Returns whether or not this piece represents a monomino (a polyomino
    /// with one mino).
    #[must_use]
    pub const fn is_monomino(self) -> bool {
        self.mino_count().get() == 1
    }

    /// Returns whether or not this piece represents a domino (a polyomino
    /// with two minos).
    #[must_use]
    pub const fn is_domino(self) -> bool {
        self.mino_count().get() == 2
    }

    /// Returns whether or not this piece represents a triomino (a polyomino
    /// with three minos).
    #[must_use]
    #[doc(alias = "is_trimino")]
    #[doc(alias = "is_tromino")]
    pub const fn is_triomino(self) -> bool {
        self.mino_count().get() == 3
    }

    /// Returns whether or not this piece represents a tetromino (a polyomino
    /// with four minos).
    #[must_use]
    #[doc(alias = "is_tetramino")]
    pub const fn is_tetromino(self) -> bool {
        self.mino_count().get() == 4
    }

    /// Returns whether or not this piece represents a pentomino (a polyomino
    /// with five minos).
    #[must_use]
    #[doc(alias = "is_pentamino")]
    pub const fn is_pentomino(self) -> bool {
        self.mino_count().get() == 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "strum")]
    const _CONST_ASSERT_TOTAL_PIECE_COUNT_MISMATCH: () = {
        use strum::EnumCount;

        assert!(
            Piece::COUNT == TOTAL_PIECE_COUNT,
            "TOTAL_PIECE_COUNT should match amount of variants in `Pieces` enum"
        );
    };

    /// Asserts that the `Piece` enum has no discontinuities and is evenly
    /// ordered
    #[test]
    #[cfg(feature = "strum")]
    fn piece_enum_has_no_holes() {
        use strum::IntoEnumIterator;

        let mut prev_piece_usize = None;
        for piece in Piece::iter() {
            let piece_usize = piece as usize;

            assert!(piece_usize < TOTAL_PIECE_COUNT);
            if let Some(prev) = prev_piece_usize {
                assert_eq!(prev + 1, piece_usize);
            }

            prev_piece_usize = Some(piece_usize);
        }
    }

    /// Asserts that every entry in the `Piece` enum is always exactly one of a
    /// monomino, domino, triomino, tetromino, or a pentomino.
    #[test]
    #[cfg(feature = "strum")]
    fn piece_enum_is_only_one_polyomino() {
        use strum::IntoEnumIterator;

        for piece in Piece::iter() {
            let mut order = None;

            let mut insert = |n: u8| {
                if let Some(old) = order.replace(n) {
                    panic!(
                        "Assertion failed: {piece} already {old} but also {n}?",
                        piece = <&str>::from(&piece)
                    );
                }
            };

            if piece.is_monomino() {
                insert(1);
            }

            if piece.is_domino() {
                insert(2);
            }

            if piece.is_triomino() {
                insert(3);
            }

            if piece.is_tetromino() {
                insert(4);
            }

            if piece.is_pentomino() {
                insert(5);
            }

            assert!(
                order.is_some(),
                "Assertion failed: {piece} order is undefined?",
                piece = <&str>::from(&piece)
            );

            assert_eq!(order, Some(piece.mino_count().get()));
        }
    }
}
