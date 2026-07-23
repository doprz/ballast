use crate::model::disk::DiskDevice;

pub trait DiskStatsProvider: Send {
    fn enumerate(&mut self) -> color_eyre::Result<Vec<DiskDevice>>;
    // fn sample_io(&mut self) -> color_eyre::Result<>;
}
