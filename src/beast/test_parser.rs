use crate::beast::parser::*;
use crate::beast::*;

// 00000000  1a 32 07 94 f8 8e 22 26  04 5d a1 1b 00 44 e9 57  |.2...."&.]...D.W|
// 00000010  1a 32 07 94 f8 8e c0 f0  34 02 81 83 16 f9 21 89  |.2......4.....!.|
// 00000020  1a 32 07 94 f8 8f c4 a0  15 02 c1 87 b9 b4 50 ae  |.2............P.|
// 00000030  1a 32 07 94 f8 8f ea d4  21 02 a1 85 3f c9 f0 7c  |.2......!...?..||
// 00000040  1a 32 07 94 f8 90 22 57  0c 02 61 81 3a c8 5b 87  |.2...."W..a.:.[.|
// 00000050  1a 32 07 94 f8 91 25 44  14 02 c1 86 be 43 49 a6  |.2....%D.....CI.|

#[test]
fn test_parse_mode_s_short() {
    let input = vec![
        0x1a, 0x32, 0x07, 0x94, 0xf8, 0x8e, 0x22, 0x26, 0x04, 0x28, 0x00, 0x1b, 0x98, 0x03, 0x82,
        0x0c,
    ];

    let parser = Parser::new();

    let (input, _message) = parser.parse(&input).unwrap();

    assert_eq!(0, input.len());
}

#[test]
fn test_parse_mode_s_long() {
    let input = vec![
        0x1a, 0x33, 0x0b, 0x5d, 0xe6, 0x66, 0x3f, 0x2e, 0x1e, 0x8d, 0xa6, 0xee, 0x47, 0x23, 0x05,
        0x30, 0x76, 0xd7, 0x48, 0x20, 0x54, 0x47, 0x7b,
    ];

    let parser = Parser::new();

    let (input, _message) = parser.parse(&input).unwrap();

    assert_eq!(0, input.len());
}

#[test]
fn test_parse_df_0() {
    let input = vec![0x02, 0x81, 0x83, 0x16, 0xf9, 0x21, 0x89];

    let data = parse_df_0(&input);

    let expected = Data::ACASSurveillanceReply(ACASSurveillanceReply {
        vertical_status: AircraftStatus::Either,
        cross_link: CrossLink::Supported,
        sensitivity_level: SensitivityLevel::Operative(4),
        reply_information: ReplyInformation::ACASVerticalOnly,
        altitude: Altitude::Feet(3950),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_4() {
    let input = vec![0x20, 0x00, 0x03, 0x97, 0xc2, 0x6e, 0x02];

    let data = parse_df_4(&input);

    let expected = Data::AltitudeReply(AltitudeReply {
        flight_status: FlightStatus {
            alert: false,
            spi: false,
            status: AircraftStatus::Airborne,
        },
        downlink_request: 0,
        utility_message: 0,
        altitude: Altitude::Feet(4775),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_5() {
    let input = vec![0x5d, 0xa1, 0x1b, 0x00, 0x44, 0xe9, 0x57];

    let data = parse_df_5(&input);

    let expected = Data::SurveillanceReply(SurveillanceReply {
        flight_status: FlightStatus {
            alert: false,
            spi: true,
            status: AircraftStatus::Either,
        },
        downlink_request: 20,
        utility_message: 8,
        id: 12368,
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17() {
    let input = vec![
        0x8d, 0xa6, 0xee, 0x47, 0x23, 0x05, 0x30, 0x76, 0xd7, 0x48, 0x20, 0x54, 0x47, 0x7b,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A6EE47".to_string(),
        message: ADSBMessage::AircraftIdentification(AircraftIdentification {
            category: AircraftCategory::Medium2,
            call_sign: "ASA654  ".to_string(),
        }),
    });

    assert_eq!(expected, data);
}
