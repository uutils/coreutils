use std::io::IoError;

enum PipelineError {
    PipelineIoError(IoError)
    OtherPipelineError(String)
}

type PipelineResult<A> = Result<A, PipelineError>;

trait Consumer {
    fn consume(&self, data: &'a [u8]) -> PipelineResult<()>;
}

trait Producer {
    fn can_produce(&self) -> bool;
    fn produce(&mut self) -> PipelineResult<Box<u8>>;
}

trait Conversion : Consumer + Producer {

}

struct FDConsumer {
    fd: &'a ddio::RawFD
}

impl FDConsumer {

}

impl Consumer for FDConsumer {
    fn consume(&mut self, data: &'a [u8]) -> PipelineResult<()> {
        try!(self.fd.write(data))
    }
}

struct FDProducer {
    fd: &'a ddio::RawFD
}
