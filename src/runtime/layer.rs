/// Base trait for all runtime layers with common initialization behavior.
///
/// This trait provides a standard interface for runtime layers (Security, Storage, Delivery)
/// and includes default implementations for common operations.
pub trait RuntimeLayer {
    /// Returns the kind/name of this layer as a static string.
    fn kind(&self) -> &'static str;

    /// Initializes the layer, returning an error if initialization fails.
    ///
    /// The default implementation logs the initialization with the layer's kind.
    /// Override this method to provide layer-specific initialization logic.
    fn initialize(&self) -> Result<(), String> {
        println!("Initializing {} layer: {}", self.layer_name(), self.kind());
        Ok(())
    }

    /// Returns the display name of the layer type for logging purposes.
    ///
    /// This is implemented by each concrete layer type (e.g., "security", "storage", "delivery").
    fn layer_name(&self) -> &'static str;
}
