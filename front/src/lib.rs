pub mod components;
pub mod listening;
pub mod status;
pub mod streaming;

static BUFFER_LENGTH: usize = 1000000;
static BUFFER_LIMIT: usize = BUFFER_LENGTH / 100 * 90;
