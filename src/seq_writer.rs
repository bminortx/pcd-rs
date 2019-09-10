//! [SeqWriter](crate::seq_writer::SeqWriter) lets you write points sequentially to
//! PCD file or writer given by user. The written point type must implement
//! [PCDRecordWrite](crate::record::PCDRecordWrite) trait.
//! See [record](crate::record) moduel doc to implement your own point type.
//!
//! ```rust
//! use failure::Fallible;
//! use pcd_rs::{DataKind, seq_writer::SeqWriterBuilder, PCDRecordWrite};
//! use std::path::Path;
//!
//! #[derive(PCDRecordWrite)]
//! pub struct Point {
//!     x: f32,
//!     y: f32,
//!     z: f32,
//! }
//!
//! fn main() -> Fallible<()> {
//!     let viewpoint = Default::default();
//!     let kind = DataKind::ASCII;
//!     let mut writer = SeqWriterBuilder::<Point>::new(300, 1, viewpoint, kind)?
//!         .create("test_files/dump.pcd")?;
//!
//!     let point = Point {
//!         x: 3.14159,
//!         y: 2.71828,
//!         z: -5.0,
//!     };
//!
//!     writer.push(&point)?;
//!
//!     Ok(())
//! }
//! ```

use crate::{record::PCDRecordWrite, DataKind, ValueKind, ViewPoint};
use failure::Fallible;
use std::{
    fs::File,
    io::{prelude::*, BufWriter, SeekFrom},
    marker::PhantomData,
    path::Path,
};

/// A builder type that builds [SeqWriter](crate::seq_writer::SeqWriter).
pub struct SeqWriterBuilder<T: PCDRecordWrite> {
    width: u64,
    height: u64,
    viewpoint: ViewPoint,
    data_kind: DataKind,
    record_spec: Vec<(String, ValueKind, usize)>,
    _phantom: PhantomData<T>,
}

impl<T: PCDRecordWrite> SeqWriterBuilder<T> {
    /// Create new [SeqWriterBuilder](crate::seq_writer::SeqWriterBuilder) that
    /// stores header data.
    pub fn new(
        width: u64,
        height: u64,
        viewpoint: ViewPoint,
        data_kind: DataKind,
    ) -> Fallible<SeqWriterBuilder<T>> {
        let record_spec = T::write_spec();

        let builder = SeqWriterBuilder {
            width,
            height,
            viewpoint,
            data_kind,
            record_spec,
            _phantom: PhantomData,
        };

        Ok(builder)
    }

    /// Builds new [SeqWriter](crate::seq_writer::SeqWriter) object from a writer.
    /// The writer must implement both [Write](std::io::Write) and [Write](std::io::Seek)
    /// traits.
    pub fn from_writer<R: Write + Seek>(self, writer: R) -> Fallible<SeqWriter<R, T>> {
        let seq_writer = SeqWriter::new(self, writer)?;
        Ok(seq_writer)
    }

    /// Builds new [SeqWriter](crate::seq_writer::SeqWriter) by creating a new file.
    pub fn create<P: AsRef<Path>>(self, path: P) -> Fallible<SeqWriter<BufWriter<File>, T>> {
        let writer = BufWriter::new(File::create(path.as_ref())?);
        let seq_writer = self.from_writer(writer)?;
        Ok(seq_writer)
    }
}

/// A Writer type that write points to PCD data.
pub struct SeqWriter<R: Write + Seek, T: PCDRecordWrite> {
    writer: R,
    builder: SeqWriterBuilder<T>,
    num_records: usize,
    points_arg_begin: u64,
    points_arg_width: usize,
}

impl<R: Write + Seek, T: PCDRecordWrite> SeqWriter<R, T> {
    fn new(builder: SeqWriterBuilder<T>, mut writer: R) -> Fallible<SeqWriter<R, T>> {
        let (points_arg_begin, points_arg_width) = Self::write_meta(&builder, &mut writer)?;
        dbg!(points_arg_begin, points_arg_width);
        let seq_writer = SeqWriter {
            builder,
            writer,
            num_records: 0,
            points_arg_begin,
            points_arg_width,
        };
        Ok(seq_writer)
    }

    fn write_meta(builder: &SeqWriterBuilder<T>, writer: &mut R) -> Fallible<(u64, usize)> {
        let fields_args = builder
            .record_spec
            .iter()
            .map(|(name, _, _)| name.to_owned())
            .collect::<Vec<_>>();

        let size_args = builder
            .record_spec
            .iter()
            .map(|(_, kind, _)| {
                use ValueKind::*;
                let size = match kind {
                    U8 | I8 => 1,
                    U16 | I16 => 2,
                    U32 | I32 | F32 => 4,
                    F64 => 8,
                };
                size.to_string()
            })
            .collect::<Vec<_>>();

        let type_args = builder
            .record_spec
            .iter()
            .map(|(_, kind, _)| {
                use ValueKind::*;
                match kind {
                    U8 | U16 | U32 => "U",
                    I8 | I16 | I32 => "I",
                    F32 | F64 => "F",
                }
            })
            .collect::<Vec<_>>();

        let count_args = builder
            .record_spec
            .iter()
            .map(|(_, _, count)| count.to_string())
            .collect::<Vec<_>>();

        let viewpoint_args = {
            let viewpoint = &builder.viewpoint;
            [
                viewpoint.tx,
                viewpoint.ty,
                viewpoint.tz,
                viewpoint.qw,
                viewpoint.qx,
                viewpoint.qy,
                viewpoint.qz,
            ]
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
        };

        let points_arg_width = (usize::max_value() as f64).log10().floor() as usize + 1;

        writeln!(writer, "# .PCD v.7 - Point Cloud Data file format")?;
        writeln!(writer, "VERSION .7")?;
        writeln!(writer, "FIELDS {}", fields_args.join(" "))?;
        writeln!(writer, "SIZE {}", size_args.join(" "))?;
        writeln!(writer, "TYPE {}", type_args.join(" "))?;
        writeln!(writer, "COUNT {}", count_args.join(" "))?;
        writeln!(writer, "WIDTH {}", builder.width)?;
        writeln!(writer, "HEIGHT {}", builder.height)?;
        writeln!(writer, "VIEWPOINT {}", viewpoint_args.join(" "))?;

        write!(writer, "POINTS ")?;
        let points_arg_begin = writer.seek(SeekFrom::Current(0))?;
        writeln!(writer, "{:width$}", " ", width = points_arg_width)?;

        match builder.data_kind {
            DataKind::Binary => writeln!(writer, "DATA binary")?,
            DataKind::ASCII => writeln!(writer, "DATA ascii")?,
        }

        Ok((points_arg_begin, points_arg_width))
    }

    /// Writes a new point to PCD data.
    pub fn push(&mut self, record: &T) -> Fallible<()> {
        match self.builder.data_kind {
            DataKind::Binary => record.write_chunk(&mut self.writer)?,
            DataKind::ASCII => record.write_line(&mut self.writer)?,
        }
        self.num_records += 1;

        let eof_pos = self.writer.seek(SeekFrom::Current(0))?;
        self.writer.seek(SeekFrom::Start(self.points_arg_begin))?;
        write!(
            self.writer,
            "{:<width$}",
            self.num_records,
            width = self.points_arg_width
        )?;
        self.writer.seek(SeekFrom::Start(eof_pos))?;

        Ok(())
    }
}