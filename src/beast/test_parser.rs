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
fn test_header_timestamp() {
    let input = vec![0x0b, 0x5d, 0xe6, 0x66, 0x3f, 0x2e];

    let (input, timestamp) = header_timestamp(&input).unwrap();

    assert_eq!(12497925324590, timestamp);
    assert_eq!(0, input.len());
}

#[test]
fn test_header_signal() {
    let input = vec![0x1e];

    let (input, signal) = header_signal(&input).unwrap();

    assert_eq!(-18.588378514285854, signal);
    assert_eq!(0, input.len());
}

#[test]
fn test_parse_df_0() {
    let input = vec![0x02, 0x81, 0x83, 0x16, 0xf9, 0x21, 0x89];

    let data = parse_df_0(&input);

    let expected = Data::ACASSurveillanceReply(ACASSurveillanceReply {
        vertical_status: VerticalStatus::Either,
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
            status: VerticalStatus::Airborne,
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
            status: VerticalStatus::Either,
        },
        downlink_request: 20,
        utility_message: 8,
        id: 12368,
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_11() {
    let input = vec![0x5d, 0xa6, 0xa6, 0xb7, 0xfd, 0xe8, 0xb1];

    let data = parse_df_11(&input);

    let expected = Data::AllCallReply(AllCallReply {
        capability: 5,
        icao: "A6A6B7".to_string(),
        parity: 16640177,
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_16() {
    let input = vec![
        0x80, 0x81, 0x83, 0x3c, 0x58, 0x1b, 0xc7, 0x05, 0x35, 0x7f, 0xfd, 0x13, 0x90, 0xfb,
    ];

    let data = parse_df_16(&input);

    let expected = Data::ACASCoordinationReply(ACASCoordinationReply {
        vertical_status: VerticalStatus::Either,
        sensitivity_level: SensitivityLevel::Operative(4),
        reply_information: ReplyInformation::ACASVerticalOnly,
        altitude: Altitude::Feet(4500),
        vds: 5774279,
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17_tc_unsupported() {
    let input = vec![
        0x8d, 0xa6, 0xee, 0x47, 0xf8, 0x23, 0x00, 0x02, 0x00, 0x49, 0xb8,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A6EE47".to_string(),
        message: ADSBMessage::Unsupported(vec![0xf8, 0x23, 0x00, 0x02, 0x00, 0x49, 0xb8]),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17_tc_4() {
    let input = vec![
        0x8d, 0xa6, 0xee, 0x47, 0x23, 0x05, 0x30, 0x76, 0xd7, 0x48, 0x20,
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

#[test]
fn test_parse_df_17_tc_11() {
    let input = vec![
        0x8d, 0xa4, 0x5f, 0xb1, 0x58, 0x0b, 0xc7, 0x26, 0x5d, 0x80, 0x06,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A45FB1".to_string(),
        message: ADSBMessage::AirbornePosition(AirbornePosition {
            surveillance_status: SurveillanceStatus::NoCondition,
            single_antenna: false,
            altitude: Altitude::Feet(500),
            utc_synchronized: false,
            cpr_format: CPRFormat::Odd,
            cpr_latitude: 103214,
            cpr_longitude: 98310,
        }),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17_tc_19_st_1() {
    let input = vec![
        0x8d, 0xa8, 0x2d, 0xfb, 0x99, 0x10, 0x6b, 0xb2, 0x70, 0x54, 0x09,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A82DFB".to_string(),
        message: ADSBMessage::Velocity(Velocity {
            intent_change: false,
            ifr_capability: false,
            navigation_uncertainty: 2,
            velocity: VelocityType::Ground(GroundVelocity {
                supersonic_aircraft: false,
                east_west_direction: EastWestDirection::WestToEast,
                east_west_velocity: 107,
                north_south_direction: NorthSouthDirection::NorthToSouth,
                north_south_velocity: 403,
            }),
            vertical_rate: VerticalRate::FeetPerMinute(VerticalRateSource::Barometer(1280)),
            altitude_difference: AltitudeDifference::Feet(225),
        }),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17_tc_19_st_3() {
    let input = vec![
        0x8d, 0xa8, 0x2d, 0xfb, 0x9b, 0x06, 0x6b, 0xb2, 0x70, 0x85, 0x02,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A82DFB".to_string(),
        message: ADSBMessage::Velocity(Velocity {
            intent_change: false,
            ifr_capability: false,
            navigation_uncertainty: 0,
            velocity: VelocityType::Airborne(Airspeed {
                supersonic_aircraft: false,
                magnetic_heading_available: true,
                magnetic_heading: 619,
                airspeed_type: AirspeedType::True,
                airspeed: 403,
            }),
            vertical_rate: VerticalRate::FeetPerMinute(VerticalRateSource::Barometer(2048)),
            altitude_difference: AltitudeDifference::Feet(50),
        }),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17_tc_28() {
    let input = vec![
        0x8d, 0xa5, 0x7d, 0x52, 0xe1, 0x1f, 0xa8, 0x00, 0x00, 0x00, 0x00,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A57D52".to_string(),
        message: ADSBMessage::AircraftStatus(AircraftStatus {
            emergency: Emergency::None,
            squawk: 29552,
        }),
    });

    assert_eq!(expected, data);
}

#[test]
fn test_parse_df_17_tc_29() {
    let input = vec![
        0x8d, 0xa8, 0x2d, 0xfb, 0xea, 0x38, 0xc8, 0x60, 0x01, 0x5f, 0x88,
    ];

    let data = parse_df_17(&input);

    let expected = Data::ExtendedSquitter(ExtendedSquitter {
        capability: 5,
        icao: "A82DFB".to_string(),
        message: ADSBMessage::TargetState(TargetStateType::SubType1(TargetState1 {
            sil_supplement: SourceIntegrityLevelSupplement::PerHour,
            altitude_source: AltitudeSource::MCPFCU,
            altitude_setting: AltitudeSetting::Feet(14528),
            barometer_setting: BarometerSetting::MilliBar(906.4),
            heading_setting: HeadingSetting::None,
            nac_position: 2,
            nic_barometric: 1,
            sil: SourceIntegrityLevel::PerThousand,
            autopilot: Some(true),
            vnav: Some(true),
            altitude_hold: Some(true),
            autopilot_approach: Some(true),
            tcas: Some(false),
            lnav: Some(false),
        })),
    });

    assert_eq!(expected, data);
}
