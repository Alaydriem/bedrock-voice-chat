use moka::sync::Cache;
use rodio::{
    OutputStreamHandle, Sink, SpatialSink
};
use std::sync::Arc;
use std::time::Duration;

pub(crate) enum AudioSinkType {
    SpatialSink,
    Sink
}

pub(crate) enum AudioSinkInner<A, B> {
    Sink(A),
    SpatialSink(B)
}

impl TryFrom<AudioSinkInner<Arc<Sink>, Arc<SpatialSink>>> for Arc<rodio::Sink> {
    type Error = ();

    fn try_from(value: AudioSinkInner<Arc<Sink>, Arc<SpatialSink>>) -> Result<Self, Self::Error> {
        match value {
            AudioSinkInner::Sink(sink) => Ok(sink),
            _ => Err(())
        }
    }
}

impl TryFrom<AudioSinkInner<Arc<Sink>, Arc<SpatialSink>>> for Arc<rodio::SpatialSink> {
    type Error = ();

    fn try_from(value: AudioSinkInner<Arc<Sink>, Arc<SpatialSink>>) -> Result<Self, Self::Error> {
        match value {
            AudioSinkInner::SpatialSink(sink) => Ok(sink),
            _ => Err(())
        }
    }
}

pub(crate) struct AudioSink {
    pub spatial_sink: Arc<SpatialSink>,
    pub sink: Arc<Sink>
}

pub(crate) struct SinkManager<'a> {
    pub sinks: Cache<String, Arc<AudioSink>>,
    handle: &'a OutputStreamHandle
}

impl<'a> SinkManager<'a> {
    pub fn new(handle: &'a OutputStreamHandle) -> Self {
        Self {
            sinks: Cache::builder()
                .time_to_idle(Duration::from_secs(15 * 60))
                .build(),
            handle
        }
    }

    pub fn get_sink(&mut self, source: String, sink_type: AudioSinkType) -> AudioSinkInner<Arc<Sink>, Arc<SpatialSink>> {
        let inner_sink = match self.sinks.get(&source.clone()) {
            Some(sink) => sink,
            None => {
                let spatial_sink = Arc::new(SpatialSink::try_new(
                    &self.handle,
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0]
                ).unwrap());
                let sink = Arc::new(Sink::try_new(&self.handle).unwrap());

                sink.play();
                spatial_sink.play();
                _ = self.sinks.insert(source.clone(), Arc::new(AudioSink {
                    spatial_sink: spatial_sink,
                    sink: sink
                }));

                self.sinks.get(&source.clone()).unwrap()
            }
        };

        match sink_type {
            AudioSinkType::Sink => AudioSinkInner::Sink(inner_sink.sink.clone()),
            AudioSinkType::SpatialSink => AudioSinkInner::SpatialSink(inner_sink.spatial_sink.clone())
        }
    }
}