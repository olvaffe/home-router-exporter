// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use std::{
    fmt::{self, Write},
    iter, time,
};

pub enum Unit {
    Bytes,
    Celsius,
    Hertz,
    Info,
    None,
    Packets,
    Seconds,
}

impl Unit {
    fn as_suffix(&self) -> &'static str {
        match self {
            Unit::Bytes => "_bytes",
            Unit::Celsius => "_celsius",
            Unit::Hertz => "_hertz",
            Unit::Info => "_info",
            Unit::None => "",
            Unit::Packets => "_packets",
            Unit::Seconds => "_seconds",
        }
    }
}

pub enum Type {
    Counter,
    Gauge,
}

impl Type {
    fn as_suffix(&self) -> &'static str {
        match self {
            Type::Counter => "_total",
            Type::Gauge => "",
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Type::Counter => "counter",
            Type::Gauge => "gauge",
        }
    }
}

pub struct Info<const N: usize> {
    pub subsys: &'static str,
    pub name: &'static str,
    pub help: &'static str,
    pub unit: Unit,
    pub ty: Type,
    pub label_keys: [&'static str; N],
}

pub struct MetricEncoder<'a, const N: usize> {
    writer: &'a mut String,
    name: String,
    label_keys: &'a [&'a str; N],
    timestamp: i64,
}

impl<'a, const N: usize> MetricEncoder<'a, N> {
    fn new(
        writer: &'a mut String,
        namespace: &str,
        info: &'a Info<N>,
        timestamp: Option<time::SystemTime>,
    ) -> Self {
        let name = format!(
            "{}_{}_{}{}{}",
            namespace,
            info.subsys,
            info.name,
            info.unit.as_suffix(),
            info.ty.as_suffix()
        );
        let label_keys = &info.label_keys;
        let timestamp = timestamp.map_or(0, |ts| {
            ts.duration_since(time::UNIX_EPOCH)
                .map_or(0, |dur| dur.as_millis() as i64)
        });

        let mut menc = MetricEncoder {
            writer,
            name,
            label_keys,
            timestamp,
        };

        menc.write_info(info);

        menc
    }

    fn write_info(&mut self, info: &Info<N>) {
        let _ = self
            .writer
            .write_fmt(format_args!("# HELP {} {}\n", self.name, info.help));
        let _ = self
            .writer
            .write_fmt(format_args!("# TYPE {} {}\n", self.name, info.ty.as_str()));
    }

    fn write_labels(&mut self, label_vals: &[&str; N]) {
        if N == 0 {
            return;
        }

        let _ = self.writer.write_char('{');

        let mut first = true;
        for (key, val) in iter::zip(self.label_keys, label_vals) {
            if first {
                first = false;
            } else {
                let _ = self.writer.write_char(',');
            }

            let _ = self.writer.write_fmt(format_args!("{}=\"", key));
            for c in val.chars() {
                let _ = match c {
                    '\\' => self.writer.write_str(r"\\"),
                    '"' => self.writer.write_str(r#"\""#),
                    '\n' => self.writer.write_str(r"\n"),
                    c => self.writer.write_char(c),
                };
            }
            let _ = self.writer.write_char('"');
        }

        let _ = self.writer.write_char('}');
    }

    pub fn write<T: fmt::Display>(&mut self, label_vals: &[&str; N], val: T) {
        let _ = self.writer.write_str(&self.name);
        self.write_labels(label_vals);

        let _ = if self.timestamp > 0 {
            self.writer
                .write_fmt(format_args!(" {} {}\n", val, self.timestamp))
        } else {
            self.writer.write_fmt(format_args!(" {}\n", val))
        };
    }
}

pub struct Encoder<'a> {
    writer: &'a mut String,
    namespace: &'a str,
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut String, namespace: &'a str) -> Self {
        Encoder { writer, namespace }
    }

    pub fn with_info<'b, const N: usize>(
        &'b mut self,
        info: &'b Info<N>,
        timestamp: Option<time::SystemTime>,
    ) -> MetricEncoder<'b, N> {
        MetricEncoder::new(self.writer, self.namespace, info, timestamp)
    }

    pub fn write<T: fmt::Display>(
        &mut self,
        info: &Info<0>,
        val: T,
        timestamp: Option<time::SystemTime>,
    ) {
        self.with_info(info, timestamp).write(&[], val);
    }
}
