use register::Flags;

mod bits {
    pub fn num(b: bool) -> u8 {
        if b {
            1
        } else {
            0
        }
    }

    pub fn bits(x: u8) -> [bool; 8] {
        [
            x & 0x01 == 0x01,
            x & 0x02 == 0x02,
            x & 0x04 == 0x04,
            x & 0x08 == 0x08,
            x & 0x10 == 0x10,
            x & 0x20 == 0x20,
            x & 0x40 == 0x40,
            x & 0x80 == 0x80,
        ]
    }

    pub fn unbits(b: [bool; 8]) -> u8 {
        num(b[7]) * 0x80
            + num(b[6]) * 0x40
            + num(b[5]) * 0x20
            + num(b[4]) * 0x10
            + num(b[3]) * 0x08
            + num(b[2]) * 0x04
            + num(b[1]) * 0x02
            + num(b[0])
    }
}

/*
    b i1 i2   o bo
    0 0  0    0 0
    0 0  1    1 1
    0 1  0    1 0
    0 1  1    0 0
    1 0  0    1 1
    1 0  1    0 1
    1 1  0    0 0
    1 1  1    1 1
*/
// returns result, was there a borrow at 4, and was there a borrow at the end
fn sub_(left: u8, right: u8) -> (u8, bool, bool) {
    use self::bits::{bits, num, unbits};

    fn lookup(b: bool, i1: bool, i2: bool) -> (bool, bool) {
        let lut = [
            (false, false),
            (true, true),
            (true, false),
            (false, false),
            (true, true),
            (false, true),
            (false, false),
            (true, true),
        ];
        let idx = num(b) * 4 + num(i1) * 2 + num(i2);
        lut[idx as usize]
    }

    let bits_left = bits(left);
    let bits_right = bits(right);
    let mut borrow = false;
    let mut borrow_at_4 = false;
    let mut result = [true, true, true, true, true, true, true, true];

    for i in 0..8 {
        let (o, bo) = lookup(borrow, bits_left[i], bits_right[i]);
        borrow = bo;
        if i == 3 {
            borrow_at_4 = bo;
        };
        result[i] = o;
    }

    (unbits(result), borrow_at_4, borrow)
}

pub fn sub(flags: &mut Flags, left: u8, right: u8) -> u8 {
    let (r, borrow_at_4, borrow_at_end) = sub_(left, right);
    flags.z = r == 0;
    flags.n = true;
    flags.h = !borrow_at_4;
    flags.c = !borrow_at_end;
    r
}

/*
    c i1 i2   o co
    0 0  0    0 0
    0 0  1    1 0
    0 1  0    1 0
    0 1  1    0 1
    1 0  0    1 0
    1 0  1    0 1
    1 1  0    0 1
    1 1  1    1 1
*/
// returns result, was there a carry at 3, and was there a carry at the end
fn add_(left: u8, right: u8) -> (u8, bool, bool) {
    use self::bits::{bits, num, unbits};

    fn lookup(c: bool, i1: bool, i2: bool) -> (bool, bool) {
        let lut = [
            (false, false),
            (true, false),
            (true, false),
            (false, true),
            (true, false),
            (false, true),
            (false, true),
            (true, true),
        ];
        let idx = num(c) * 4 + num(i1) * 2 + num(i2);
        lut[idx as usize]
    }

    let bits_left = bits(left);
    let bits_right = bits(right);
    let mut carry = false;
    let mut carry_at_3 = false;
    let mut result = [true, true, true, true, true, true, true, true];

    for i in 0..8 {
        let (o, co) = lookup(carry, bits_left[i], bits_right[i]);
        println!("Out {}, co {} at {}", o, co, i);
        carry = co;
        if i == 3 {
            carry_at_3 = co;
        };
        result[i] = o;
    }

    (unbits(result), carry_at_3, carry)
}

pub fn add(flags: &mut Flags, left: u8, right: u8) -> u8 {
    let (r, carry_at_3, carry_at_end) = add_(left, right);
    flags.z = r == 0;
    flags.n = false;
    flags.h = carry_at_3;
    flags.c = carry_at_end;
    r
}

pub fn xor(flags: &mut Flags, left: u8, right: u8) -> u8 {
    let r = left ^ right;
    flags.reset();
    flags.z = r == 0;
    r
}

pub fn inc(flags: &mut Flags, x: u8) -> u8 {
    let r = x.wrapping_add(1);
    flags.z = r == 0;
    flags.n = false;
    flags.h = x & 0b111 == 0b111;
    // c not affected
    r
}

pub fn dec(flags: &mut Flags, x: u8) -> u8 {
    let old_c = flags.c;
    let r = sub(flags, x, 1);
    flags.c = old_c;
    r
}

pub fn inc16(_flags: &mut Flags, x: u16) -> u16 {
    x.wrapping_add(1)
}

pub fn dec16(_flags: &mut Flags, x: u16) -> u16 {
    x.wrapping_sub(1)
}

pub fn rl(flags: &mut Flags, x: u8) -> u8 {
    let r = (x << 1) | (if flags.c { 1 } else { 0 });
    flags.reset();
    flags.z = r == 0;
    flags.c = x & 0x80 == 0x80;
    r
}

pub fn bit(flags: &mut Flags, x: u8, b: u8) {
    assert!(b < 8);
    flags.z = x & (1 << b) == 0;
    flags.n = false;
    flags.h = true;
    // c not affected
}

#[cfg(test)]
mod tests {
    use alu;
    use test::proptest::prelude::*;

    #[test]
    fn borrow_at_4() {
        let (_, b4, _) = alu::sub_(0b0111, 0b1000);
        assert!(b4);
    }

    #[test]
    fn borrow_at_end() {
        let (_, _, b) = alu::sub_(4, 5);
        assert!(b);
    }

    #[test]
    fn carry_at_3() {
        let (_, c3, _) = alu::add_(0b1000, 0b1000);
        assert!(c3);
    }

    #[test]
    fn carry_at_end() {
        let (_, _, c) = alu::add_(0xff, 0xff);
        assert!(c);
    }

    proptest! {
        #[test]
        fn bits_unbits_selfinverse(x : u8) {
            assert_eq!(alu::bits::unbits(alu::bits::bits(x)), x)
        }

        #[test]
        fn subtraction_works(left : u8, right: u8) {
            let (r, _, _) = alu::sub_(left, right);
            assert_eq!(left.wrapping_sub(right), r)
        }

        #[test]
        fn addition_works(left: u8, right: u8) {
            let (r, _, _) = alu::add_(left, right);
            assert_eq!(left.wrapping_add(right), r)
        }
    }
}
