// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use std::{
    fmt::{self, Write},
    iter,
};

pub enum Unit {
    Bytes,
    Celsius,
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

pub struct MetricEncoder<'a, 'b, const N: usize> {
    writer: &'a mut String,
    name: String,
    label_keys: &'b [&'static str; N],
}

impl<'a, 'b, const N: usize> MetricEncoder<'a, 'b, N> {
    fn new(writer: &'a mut String, namespace: &str, info: &'b Info<N>) -> Self {
        let name = format!(
            "{}_{}_{}{}{}",
            namespace,
            info.subsys,
            info.name,
            info.unit.as_suffix(),
            info.ty.as_suffix()
        );
        let mut menc = MetricEncoder {
            writer,
            name,
            label_keys: &info.label_keys,
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

            let _ = self.writer.write_fmt(format_args!("{}=\"{}\"", key, val));
        }

        let _ = self.writer.write_char('}');
    }

    pub fn write<T: fmt::Display>(&mut self, label_vals: &[&str; N], val: T) {
        let _ = self.writer.write_str(&self.name);
        self.write_labels(label_vals);
        let _ = self.writer.write_fmt(format_args!(" {}\n", val));
    }
}

pub struct Encoder {
    writer: String,
    namespace: &'static str,
}

impl Encoder {
    pub fn new(namespace: &'static str) -> Self {
        Encoder {
            writer: String::new(),
            namespace,
        }
    }

    pub fn with_info<'a, 'b, const N: usize>(
        &'a mut self,
        info: &'b Info<N>,
    ) -> MetricEncoder<'a, 'b, N> {
        MetricEncoder::new(&mut self.writer, self.namespace, info)
    }

    pub fn write<T: fmt::Display>(&mut self, info: &Info<0>, val: T) {
        self.with_info(info).write(&[], val);
    }

    pub fn into_string(self) -> String {
        self.writer
    }
}
