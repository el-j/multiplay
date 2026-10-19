//! DJ track analysis cache in the library database (TASK-204).

#![forbid(unsafe_code)]

use crate::RedbStorageAdapter;
use amp_domain::TrackAnalysis;
use amp_ports::AnalysisCachePort;
use redb::TableDefinition;

pub(crate) const ANALYSIS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("dj_analysis_v1");

impl AnalysisCachePort for RedbStorageAdapter {
    fn load(&self, key: &str) -> Option<TrackAnalysis> {
        let txn = self.db.begin_read().ok()?;
        let table = txn.open_table(ANALYSIS_TABLE).ok()?;
        let bytes = table.get(key).ok()??;
        bincode::deserialize(bytes.value()).ok()
    }

    fn store(&self, key: &str, analysis: &TrackAnalysis) -> Result<(), String> {
        let bytes = bincode::serialize(analysis).map_err(|e| e.to_string())?;
        let txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = txn.open_table(ANALYSIS_TABLE).map_err(|e| e.to_string())?;
            table
                .insert(key, bytes.as_slice())
                .map_err(|e| e.to_string())?;
        }
        txn.commit().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::ANALYSIS_VERSION;

    #[test]
    fn analysis_survives_reopening_the_database() {
        let dir = std::env::temp_dir().join(format!("multiplay-analysis-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("library.redb");
        let a = TrackAnalysis {
            version: ANALYSIS_VERSION,
            duration_secs: 312.5,
            bpm: Some(126.0),
            first_downbeat_secs: 0.42,
            beats_per_bar: 4,
            overview: vec![1, 2, 3],
            detail: vec![[10, 20, 30]],
            detail_rate: 100.0,
        };
        {
            let db = RedbStorageAdapter::new(&path).unwrap();
            assert!(db.load("file:/x.flac").is_none());
            db.store("file:/x.flac", &a).unwrap();
        }
        let db = RedbStorageAdapter::new(&path).unwrap();
        assert_eq!(db.load("file:/x.flac"), Some(a));
        let _ = std::fs::remove_dir_all(dir);
    }
}
