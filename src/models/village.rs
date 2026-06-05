use serde::{Deserialize, Serialize};
use wilayah::Village;

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct VillageRow {
    pub kode: String,
    pub nama: String,
    pub kecamatan: String,
    pub kota: String,
    pub provinsi: String,
    pub lat: f64,
    pub lon: f64,
}

impl From<&VillageRow> for Village {
    fn from(r: &VillageRow) -> Self {
        Village {
            code: r.kode.clone(),
            name: r.nama.clone(),
            district: r.kecamatan.clone(),
            city: r.kota.clone(),
            province: r.provinsi.clone(),
            lat: r.lat,
            lon: r.lon,
            dist_km: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_village_row_to_village_conversion() {
        let row = VillageRow {
            kode: "31.71.01.1001".to_string(),
            nama: "Cempaka Putih".to_string(),
            kecamatan: "Cempaka Putih".to_string(),
            kota: "Jakarta Pusat".to_string(),
            provinsi: "DKI Jakarta".to_string(),
            lat: -6.175,
            lon: 106.865,
        };

        let village: Village = Village::from(&row);
        assert_eq!(village.code, "31.71.01.1001");
        assert_eq!(village.name, "Cempaka Putih");
        assert_eq!(village.district, "Cempaka Putih");
        assert_eq!(village.city, "Jakarta Pusat");
        assert_eq!(village.province, "DKI Jakarta");
        assert!((village.lat - (-6.175)).abs() < 0.001);
        assert!((village.lon - 106.865).abs() < 0.001);
        assert!(village.dist_km.is_none());
    }
}
