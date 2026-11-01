#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate};

    use crate::layout::{decode_date, scale_attribute, stored_attribute_index, trait_name};

    #[test]
    fn date_round_trips_day_of_year() {
        let mut bytes = [0u8; 4];
        let day = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
        let doy = day.ordinal() as u16;
        let packed = doy;
        bytes[..2].copy_from_slice(&packed.to_le_bytes());
        bytes[2..].copy_from_slice(&2026u16.to_le_bytes());
        assert_eq!(decode_date(&bytes, 0), Some(day));
    }

    #[test]
    fn date_rejects_bad_years_and_days() {
        let mut bytes = [0u8; 4];
        bytes[..2].copy_from_slice(&1u16.to_le_bytes());
        bytes[2..].copy_from_slice(&1899u16.to_le_bytes());
        assert_eq!(decode_date(&bytes, 0), None);
        bytes[2..].copy_from_slice(&2026u16.to_le_bytes());
        bytes[..2].copy_from_slice(&400u16.to_le_bytes());
        assert_eq!(decode_date(&bytes, 0), None);
    }

    #[test]
    fn attributes_use_the_verified_scale() {
        assert_eq!(scale_attribute(0), 1);
        assert_eq!(scale_attribute(1), 1);
        assert_eq!(scale_attribute(50), 10);
        assert_eq!(scale_attribute(75), 15);
        assert_eq!(scale_attribute(100), 20);
    }

    #[test]
    fn attribute_indices_skip_the_two_foot_bytes() {
        assert_eq!(stored_attribute_index("crossing"), Some(0));
        assert_eq!(stored_attribute_index("technique"), Some(23));
        assert_eq!(stored_attribute_index("flair"), Some(26));
        assert_eq!(stored_attribute_index("consistency"), Some(44));
        assert_eq!(stored_attribute_index("important_matches"), Some(47));
        assert_eq!(stored_attribute_index("concentration"), Some(53));
        assert_eq!(stored_attribute_index("not_an_attribute"), None);
    }

    #[test]
    fn player_traits_cover_every_used_bit() {
        for bit in 0..64u8 {
            if bit == 53 {
                assert_eq!(trait_name(bit), None);
                continue;
            }
            assert!(trait_name(bit).is_some(), "trait bit {bit} has no name");
        }
        assert_eq!(trait_name(10), Some("Places Shots"));
        assert_eq!(trait_name(51), Some("Runs With Ball Often"));
    }
}
