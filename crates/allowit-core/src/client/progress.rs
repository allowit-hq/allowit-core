//! Progress of long provider steps. A provider reports each step; the host decides where the
//! text goes. With no sink installed, a report does nothing, so a server stays silent.
use std::sync::{Arc, RwLock};

type Sink = Arc<dyn Fn(&str) + Send + Sync>;
static SINK: RwLock<Option<Sink>> = RwLock::new(None);

/// Install the process-wide progress sink, or remove it with `None`. A terminal host writes each
/// line to its diagnostic stream.
pub fn set_sink(sink: Option<Sink>) {
    *SINK
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = sink;
}

/// Report one step, such as `Submitting the transfer`. The text holds no secret and no newline.
pub fn report(step: &str) {
    let sink = SINK
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if let Some(sink) = sink {
        sink(&super::output::one_line(step));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[test]
    fn reports_reach_only_an_installed_sink_as_one_line() {
        report("no sink installed");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let copy = seen.clone();
        set_sink(Some(Arc::new(move |step| {
            copy.lock().unwrap().push(step.to_owned())
        })));
        report("Submitting");
        report("two\nlines");
        set_sink(None);
        report("removed");
        assert_eq!(*seen.lock().unwrap(), ["Submitting", "\"two\\nlines\""]);
    }
}
