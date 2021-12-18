use crate::beast::Data;
use crate::beast::FlightStatus;
use crate::beast::Message;
use crate::beast::ModeS;
use crate::beast::Source;
use crate::beast::SurveillanceReply;

use log::debug;

use nom::bytes::streaming::*;
use nom::combinator::*;
use nom::error::*;
use nom::sequence::*;
use nom::IResult;

type VE<'a> = VerboseError<&'a [u8]>;

const MODE_AC_LENGTH: usize = 2;
const MODE_S_SHORT_LENGTH: usize = 7;
const MODE_S_LONG_LENGTH: usize = 14;

const ID_PATTERN: [(u16, u16); 12] = [
    (0x1000, 0x0010),
    (0x0800, 0x1000),
    (0x0400, 0x0020),
    (0x0200, 0x2000),
    (0x0100, 0x0040),
    (0x0080, 0x4000),
    (0x0020, 0x0100),
    (0x0010, 0x0001),
    (0x0008, 0x0200),
    (0x0004, 0x0002),
    (0x0002, 0x0400),
    (0x0001, 0x0004),
];

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse<'a>(&'a self, input: &'a [u8]) -> IResult<&'a [u8], Message, VE> {
        parse::<VE>(input)
    }
}

fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Message, E> {
    let (input, _) = tag(b"\x1a")(input)?;

    let (input, message_format) = take(1usize)(input)?;
    debug!(
        "message_format: {}",
        std::str::from_utf8(&message_format).unwrap()
    );

    let message_length = match message_format {
        b"1" => MODE_AC_LENGTH,
        b"2" => MODE_S_SHORT_LENGTH,
        b"3" => MODE_S_LONG_LENGTH,
        v => panic!("unsupported: {:?}", v),
    };
    debug!("message_length: {}", message_length);

    let (input, ts) = take_while_m_n(6, 6, |c| true)(input)?;
    let timestamp = ts.iter().fold(0u32, |ts, c| (ts << 8) | *c as u32);
    debug!("timestamp: {}", timestamp);

    let (input, signal) = take(1usize)(input)?;
    let signal = signal[0] as f64 / 255.0;
    let signal = signal * signal;
    debug!("signal: {}", signal);

    let (input, message) = take_while_m_n(message_length, message_length, |c| true)(input)?;

    debug!("message: {:?}", message);

    if message_length == MODE_AC_LENGTH {
        return Ok((
            input,
            Message::Unsupported("Mode A/C not supported".to_string()),
        ));
    }

    let (_, message) = mode_s(timestamp, signal, message).unwrap();

    Ok((input, message))
}

fn mode_s(timestamp: u32, signal_level: f64, input: &[u8]) -> IResult<&[u8], Message> {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, message_type): (_, u8) =
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(4usize))(input).unwrap();

    dbg!(message_type);

    let source = match message_type {
        0 | 4 | 5 | 16 => Source::ModeS,
        11 => Source::ModeS, // dump1090-fa uses SOURCE_MODE_S_CHECKED, but we don't check yet
        17 | 18 => Source::Adsb,
        20 | 21 => Source::Adsb,
        24..=31 => Source::ModeS,
        _ => {
            return Ok((
                input,
                Message::Unsupported(format!("unsupported message type {}", message_type)),
            ));
        }
    };

    let data = match message_type {
        5 => Message::ModeS(ModeS {
            timestamp,
            signal_level,
            data: parse_df_5(input),
        }),
        _ => panic!("message type {} not supported", message_type),
    };

    Ok((input, data))
}

pub(crate) fn parse_df_5(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, surveillance_reply) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(3usize), flight_status)),
            take(5usize),
            take(6usize),
            map(take(13usize), ident),
        )),
        |(flight_status, downlink_request, utility_message, id)| SurveillanceReply {
            flight_status,
            downlink_request,
            utility_message,
            id,
        },
    ))(input)
    .unwrap();

    Data::SurveillanceReply(surveillance_reply)
}

fn aa(input: &[u8], message_type: u8) -> Option<u32> {
    match message_type {
        11 | 17 | 18 => Some(input[1..3].iter().fold(0u32, |ts, c| (ts << 8) | *c as u32)),
        _ => None,
    }
}

// FS
fn flight_status(fs: u8) -> FlightStatus {
    match fs {
        0 => FlightStatus::Uncertain,
        1 => FlightStatus::Ground,
        2 => FlightStatus::Uncertain, // also, alert?
        3 => FlightStatus::Ground,    // also, alert?
        4 => FlightStatus::Uncertain, // also alert? spi?
        5 => FlightStatus::Uncertain, // also spi?
        _ => unreachable!("BUG: Unknown flight status {}", fs),
    }
}

// ID
fn ident(id: u16) -> u16 {
    decode(ID_PATTERN, id)
}

fn decode(pattern: [(u16, u16); 12], encoded: u16) -> u16 {
    pattern
        .iter()
        .map(|(in_bit, out_bit)| {
            if *in_bit == encoded & in_bit {
                *out_bit
            } else {
                0
            }
        })
        .fold(0, |acc, v| acc | v)
}
