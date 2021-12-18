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
        0x1a, 0x32, 0x07, 0x94, 0xf8, 0x8e, 0x22, 0x26, 0x04, 0x5d, 0xa1, 0x1b, 0x00, 0x44, 0xe9,
        0x57,
    ];

    let parser = Parser::new();

    let (input, message) = parser.parse(&input).unwrap();

    assert_eq!(0, input.len());
}

#[test]
fn test_parse_df_5() {
    let input = vec![0x5d, 0xa1, 0x1b, 0x00, 0x44, 0xe9, 0x57];

    let parser = Parser::new();

    let data = parse_df_5(&input);

    let expected = Data::SurveillanceReply(SurveillanceReply {
        flight_status: FlightStatus::Uncertain,
        downlink_request: 20,
        utility_message: 8,
        id: 12368,
    });

    assert_eq!(expected, data);
}
