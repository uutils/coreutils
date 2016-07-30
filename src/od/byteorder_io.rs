// workaround until https://github.com/BurntSushi/byteorder/issues/41 has been fixed
// based on: https://github.com/netvl/immeta/blob/4460ee/src/utils.rs#L76

use byteorder::{NativeEndian, LittleEndian, BigEndian};
use byteorder::ByteOrder as ByteOrderTrait;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    Little,
    Big,
    Native,
}

macro_rules! gen_byte_order_ops {
    ($($read_name:ident, $write_name:ident -> $tpe:ty),+) => {
        impl ByteOrder {
            $(
            #[allow(dead_code)]
            #[inline]
            pub fn $read_name(self, source: &[u8]) -> $tpe {
                match self {
                    ByteOrder::Little => LittleEndian::$read_name(source),
                    ByteOrder::Big => BigEndian::$read_name(source),
                    ByteOrder::Native => NativeEndian::$read_name(source),
                }
            }

            #[allow(dead_code)]
            pub fn $write_name(self, target: &mut [u8], n: $tpe) {
                match self {
                    ByteOrder::Little => LittleEndian::$write_name(target, n),
                    ByteOrder::Big => BigEndian::$write_name(target, n),
                    ByteOrder::Native => NativeEndian::$write_name(target, n),
                }
            }
            )+
        }
    }
}

gen_byte_order_ops! {
    read_u16, write_u16 -> u16,
    read_u32, write_u32 -> u32,
    read_u64, write_u64 -> u64,
    read_i16, write_i16 -> i16,
    read_i32, write_i32 -> i32,
    read_i64, write_i64 -> i64,
    read_f32, write_f32 -> f32,
    read_f64, write_f64 -> f64
}
