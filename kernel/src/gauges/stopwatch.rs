use bevy::platform::collections::HashMap;
use chrono::DateTime;
use std::fmt::Display;

type Marker = String;

#[derive(Debug, Clone)]
struct Reading {
    marker: Marker,
    timestamp: DateTime<chrono::Utc>,
    parent: Option<Marker>,
}

impl Reading {
    pub fn new(marker: Marker, timestamp: DateTime<chrono::Utc>, parent: Option<Marker>) -> Self {
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

    pub fn read<Marker: Into<String>>(&mut self, marker: Marker) -> DateTime<chrono::Utc> {
        self.read_to(marker.into(), self.parents.last().cloned())
    }

    pub fn read_to<Marker: Into<String>>(
        &mut self,
        marker: Marker,
        parent: Option<Marker>,
    ) -> DateTime<chrono::Utc> {
        let time = chrono::Utc::now();

        self.readings
            .push(Reading::new(marker.into(), time, parent.map(|x| x.into())));
        time
    }

    pub fn new_parent<M: Into<String>>(&mut self, marker: M) -> Option<Marker> {
        let last = self.parents.last().cloned();
        self.parents.push(marker.into());
        last
    }

    pub fn get_parent(&self, idx: usize) -> Option<Marker> {
        self.parents.get(idx).cloned()
    }
    pub fn readings(&self) -> HashMap<Marker, Vec<Reading>> {
        let mut parents = self
            .parents
            .iter()
            .map(|x| (x.clone(), Vec::new()))
            .collect::<HashMap<_, _>>();

        for r in self.readings.iter() {
            if let Some(parent) = &r.parent {
                parents.get_mut(parent).map(|x| x.push(r.clone()));
            } else {
                parents.insert(r.marker.clone(), vec![r.clone()]);
            }
        }

        parents
    }
}

impl Display for Stopwatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Stopwatch");
        let readings = self.readings();
        for (marker, readings) in readings.iter() {
            writeln!(f, " - Marker: {},", marker);
            for reading in readings {
                writeln!(f, " -- {}: {},", reading.marker, reading.timestamp);
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
        
        s.read_to("d", None);
        
        s.new_parent("b");
        
        s.read("e");

        print!("{}", s);
        
    }
}
