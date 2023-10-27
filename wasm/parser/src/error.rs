use thiserror::Error;

#[derive(Error, Debug)]
pub enum XmlError {
    #[error("no process definition found in provided xml")]
    NoProcessDefinition,
    #[error(transparent)]
    XmlDeserializeError(#[from] quick_xml::DeError),
    #[error(transparent)]
    XmlReadError(#[from] quick_xml::Error),
}
