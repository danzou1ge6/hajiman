use super::*;

fn gcd(mut a: usize, mut b: usize) -> usize {
    assert!(a != 0 && b != 0, "Inputs must be non-zero");
    while b != 0 {
        if b < a {
            std::mem::swap(&mut a, &mut b);
        }
        b %= a;
    }
    a
}

fn pad(bytes: &[u8], n_bits: u32) -> impl Iterator<Item = u8> {
    let gcd = gcd(8, n_bits as usize);
    let lcm = n_bits as usize / gcd;
    let pad_to = (bytes.len() + lcm - 1) / lcm * lcm;
    bytes
        .iter()
        .cloned()
        .chain(std::iter::repeat_n(0, pad_to - bytes.len()))
}

mod bits8 {
    use super::*;

    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
    )]
    pub struct Bits8(u8);

    impl Seq for Bits8 {
        fn prev(&self) -> Option<Self> {
            Some(Self(self.0.checked_sub(1)?))
        }

        fn succ(&self) -> Option<Self> {
            Some(Self(self.0.checked_add(1)?))
        }
    }

    impl From<u8> for Bits8 {
        fn from(value: u8) -> Self {
            Self(value)
        }
    }

    impl Bits for Bits8 {
        const N: u32 = 8;

        fn iter_bytes(arr: &[u8]) -> Padded<impl Iterator<Item = Self>> {
            Padded {
                data: arr.iter().cloned().map(Self),
                original_length: arr.len(),
            }
        }

        fn concat<E>(
            it: impl Iterator<Item = Result<Self, E>>,
            mut writer: impl std::io::Write,
        ) -> Result<(), ConcatError<E>> {
            for x in it {
                let x = x?;
                writer.write(&[x.0]).map_err(|e| ConcatError::Io(e))?;
            }
            Ok(())
        }

        fn to_usize(self) -> usize {
            self.0.into()
        }

        fn biggest() -> Self {
            Self(u8::MAX)
        }

        fn zero() -> Self {
            Self(0)
        }
    }

    impl From<Bits8> for u8 {
        fn from(value: Bits8) -> Self {
            value.0
        }
    }
}
pub use bits8::Bits8;

mod bits6 {
    use super::*;

    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
    )]
    pub struct Bits6(u8);

    impl Seq for Bits6 {
        fn prev(&self) -> Option<Self> {
            Some(Self(self.0.checked_sub(1)?))
        }

        fn succ(&self) -> Option<Self> {
            if self.0 == 63 {
                None
            } else {
                Some(Self(self.0 + 1))
            }
        }
    }

    impl From<u8> for Bits6 {
        fn from(value: u8) -> Self {
            if value <= 63 {
                Self(value)
            } else {
                panic!("{} is too large for u6", value)
            }
        }
    }

    impl Bits for Bits6 {
        const N: u32 = 6;

        fn iter_bytes(arr: &[u8]) -> Padded<impl Iterator<Item = Self>> {
            let it = pad(arr, Self::N)
                .array_chunks::<3>()
                .flat_map(|[x0, x1, x2]| {
                    [
                        x0 >> 2,
                        ((x0 & 0b00000011) << 4) | (x1 >> 4),
                        ((x1 & 0b00001111) << 2) | (x2 >> 6),
                        x2 & 0b00111111,
                    ]
                    .into_iter()
                })
                .map(Self);
            Padded {
                data: it,
                original_length: arr.len(),
            }
        }

        fn concat<E>(
            it: impl Iterator<Item = Result<Self, E>>,
            mut writer: impl std::io::Write,
        ) -> Result<(), ConcatError<E>> {
            for [x0, x1, x2, x3] in it.array_chunks::<4>() {
                let [x0, x1, x2, x3] = [x0?, x1?, x2?, x3?];
                let xs = [
                    (x0.0 << 2) | (x1.0 >> 4),
                    ((x1.0 & 0b00001111) << 4) | (x2.0 >> 2),
                    ((x2.0 & 0b00000011) << 6) | x3.0,
                ];

                writer.write(&xs).map_err(ConcatError::Io)?;
            }
            Ok(())
        }

        fn to_usize(self) -> usize {
            self.0.into()
        }

        fn biggest() -> Self {
            Self(63)
        }

        fn zero() -> Self {
            Self(0)
        }
    }

    impl From<Bits6> for u8 {
        fn from(value: Bits6) -> Self {
            value.0
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_iter_bytes() {
            let bytes = [0b0100_1001, 0b1011_0110, 0b0011_0010, 0b1110_1010];
            let Padded {
                data,
                original_length,
            } = Bits6::iter_bytes(&bytes);
            assert_eq!(
                data.map(|x| x.0).collect::<Vec<_>>(),
                vec![
                    0b010010, 0b011011, 0b011000, 0b110010, 0b111010, 0b100000, 0b000000, 0b000000
                ]
            );
            assert_eq!(original_length, 4);
        }

        #[test]
        fn test_concat_bits() {
            let bits = [
                0b010010, 0b011011, 0b011000, 0b110010, 0b111010, 0b100000, 0b000000, 0b000000,
            ]
            .into_iter()
            .map(Bits6::from)
            .map(|x| Ok::<_, ()>(x));

            let mut bytes = Vec::new();
            Bits6::concat(bits.into_iter(), &mut bytes).unwrap();
            assert_eq!(
                &bytes[..4],
                &[0b0100_1001, 0b1011_0110, 0b0011_0010, 0b1110_1010]
            );
        }
    }
}
pub use bits6::Bits6;

mod bits4 {
    use super::*;

    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
    )]
    pub struct Bits4(u8);

    impl Seq for Bits4 {
        fn prev(&self) -> Option<Self> {
            Some(Self(self.0.checked_sub(1)?))
        }

        fn succ(&self) -> Option<Self> {
            if self.0 == 15 {
                None
            } else {
                Some(Self(self.0 + 1))
            }
        }
    }

    impl From<u8> for Bits4 {
        fn from(value: u8) -> Self {
            if value <= 15 {
                Self(value)
            } else {
                panic!("{} is too large for u4", value)
            }
        }
    }

    impl Bits for Bits4 {
        const N: u32 = 4;

        fn iter_bytes(arr: &[u8]) -> Padded<impl Iterator<Item = Self>> {
            Padded {
                data: arr
                    .iter()
                    .flat_map(|byte| [*byte >> 4, *byte & 0b1111].into_iter())
                    .map(Self),
                original_length: arr.len(),
            }
        }

        fn concat<E>(
            it: impl Iterator<Item = Result<Self, E>>,
            mut writer: impl std::io::Write,
        ) -> Result<(), ConcatError<E>> {
            for eles in it.array_chunks::<2>() {
                let [x0, x1] = eles;
                let byte = x0?.0 << 4 | x1?.0;
                writer.write(&[byte]).map_err(|e| ConcatError::Io(e))?;
            }
            Ok(())
        }

        fn to_usize(self) -> usize {
            self.0.into()
        }

        fn biggest() -> Self {
            Self(15)
        }

        fn zero() -> Self {
            Self(0)
        }
    }

    impl From<Bits4> for u8 {
        fn from(value: Bits4) -> Self {
            value.0
        }
    }
}
pub use bits4::Bits4;
