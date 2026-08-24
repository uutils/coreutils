// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    Little,
    Big,
    Native,
}

macro_rules! gen_byte_order_ops {
    ($($read_name:ident -> $tpe:ty),+) => {
        impl ByteOrder {
            $(
            #[inline]
            pub fn $read_name(self, source: &[u8]) -> $tpe {
                let bytes = source[..std::mem::size_of::<$tpe>()].try_into().unwrap();
                match self {
                    Self::Little => <$tpe>::from_le_bytes(bytes),
                    Self::Big => <$tpe>::from_be_bytes(bytes),
                    Self::Native => <$tpe>::from_ne_bytes(bytes),
                }
            }
            )+
        }
    }
}

gen_byte_order_ops! {
    read_u16 -> u16,
    read_u32 -> u32,
    read_u64 -> u64,
    read_f32 -> f32,
    read_f64 -> f64,
    read_u128 -> u128
}
