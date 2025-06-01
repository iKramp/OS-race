use traits::*;

pub const LOCAL0_OP: u8 = 0x60;
const LOCAL1_OP: u8 = 0x61;
const LOCAL2_OP: u8 = 0x62;
const LOCAL3_OP: u8 = 0x63;
const LOCAL4_OP: u8 = 0x64;
const LOCAL5_OP: u8 = 0x65;
const LOCAL6_OP: u8 = 0x66;
pub const LOCAL7_OP: u8 = 0x67;

pub const ARG0_OP: u8 = 0x68;
const ARG1_OP: u8 = 0x69;
const ARG2_OP: u8 = 0x6A;
const ARG3_OP: u8 = 0x6B;
const ARG4_OP: u8 = 0x6C;
const ARG5_OP: u8 = 0x6D;
pub const ARG6_OP: u8 = 0x6E;

#[derive(Debug)]
pub enum ArgObj {
    Arg0,
    Arg1,
    Arg2,
    Arg3,
    Arg4,
    Arg5,
    Arg6,
}

impl AmlNew for ArgObj {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            ARG0_OP => Some((Self::Arg0, 1)),
            ARG1_OP => Some((Self::Arg1, 1)),
            ARG2_OP => Some((Self::Arg2, 1)),
            ARG3_OP => Some((Self::Arg3, 1)),
            ARG4_OP => Some((Self::Arg4, 1)),
            ARG5_OP => Some((Self::Arg5, 1)),
            ARG6_OP => Some((Self::Arg6, 1)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum LocalObj {
    Local0,
    Local1,
    Local2,
    Local3,
    Local4,
    Local5,
    Local6,
    Local7,
}

impl AmlNew for LocalObj {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        match data[0] {
            LOCAL0_OP => Some((Self::Local0, 1)),
            LOCAL1_OP => Some((Self::Local1, 1)),
            LOCAL2_OP => Some((Self::Local2, 1)),
            LOCAL3_OP => Some((Self::Local3, 1)),
            LOCAL4_OP => Some((Self::Local4, 1)),
            LOCAL5_OP => Some((Self::Local5, 1)),
            LOCAL6_OP => Some((Self::Local6, 1)),
            LOCAL7_OP => Some((Self::Local7, 1)),
            _ => None,
        }
    }
}
