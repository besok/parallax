use bevy::platform::collections::HashMap;
use chrono::DateTime;
use std::fmt::Display;

type Marker = String;

#[derive(Debug, Clone)]
struct Reading {
    marker: Marker,
    timestamp: DateTime<chrono::Utc>,
    parent: Marker,
}

impl Reading {
    pub fn new(marker: Marker, timestamp: DateTime<chrono::Utc>, parent: Marker) -> Self {
        Self {
            marker,
            timestamp,
            parent,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct Stopwatch {
    readings: Vec<Reading>,
    parents: Vec<Marker>,
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            readings: Vec::new(),
            parents: Vec::new(),
        }
    }

    pub fn read<M: Into<String>>(&mut self, marker: M) -> DateTime<chrono::Utc> {
        let marker = marker.into();
        let parent = self.parents.last().unwrap_or(&marker);
        self.read_to(marker.clone(), parent.clone())
    }

    pub fn read_start<Marker: Into<String>>(&mut self, marker: Marker) -> DateTime<chrono::Utc> {
        let marker = marker.into();
        self.read_to(marker.clone(), marker)
    }

    pub fn read_to<Marker: Into<String>>(
        &mut self,
        marker: Marker,
        parent: Marker,
    ) -> DateTime<chrono::Utc> {
        let time = chrono::Utc::now();

        self.readings
            .push(Reading::new(marker.into(), time, parent.into()));
        time
    }

    pub fn new_parent<M: Into<String>>(&mut self, marker: M) -> Option<Marker> {
        let last = self.parents.last().cloned();
        self.parents.push(marker.into());
        last
    }

    pub fn get_parent(&self, idx: usize) -> Option<&Marker> {
        self.parents.get(idx)
    }
    pub fn readings(&self) -> HashMap<Marker, Vec<Reading>> {
        let mut parents = HashMap::new();

        for r in self.readings.iter() {
            parents
                .entry(r.parent.clone())
                .and_modify(|e: &mut Vec<_>| e.push(r.clone()))
                .or_insert(vec![r.clone()]);
        }

        parents
    }
}

impl Display for Stopwatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = writeln!(f, "Stopwatch");
        let readings = self.readings();
        for (marker, readings) in readings.iter() {
            let _ = writeln!(f, " - Marker: {},", marker);
            for reading in readings {
                let _ = writeln!(f, "  - {}: {},", reading.marker, reading.timestamp);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::gauges::stopwatch::Stopwatch;

    #[test]
    fn stopwatch() {
        let mut s = Stopwatch::new();

        s.read("a");
        s.new_parent("a");
        s.read("b");
        s.read("c");

        s.read_start("d");

        s.new_parent("b");

        s.read("e");

        print!("{}", s);
    }
}
