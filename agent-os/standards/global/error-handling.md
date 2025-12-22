## Error handling best practices

Exceptions are a major source of complexity—each one creates additional code paths that are rarely tested. Treat exceptions as truly exceptional.

### Reduce errors at the source

- **Define errors out of existence**: Design APIs so exceptional conditions can't occur. Example: a substring operation that adjusts out-of-bounds indices to valid ranges rather than throwing.
- **Mask errors internally**: Handle errors at the lowest level possible rather than exposing them to callers. If lower-level code can reasonably recover, it should.
- **Treat most "errors" as normal flow**: Many exceptional conditions are actually expected cases—handle them as part of normal control flow, not as exceptions.

### When errors are unavoidable

- **Aggregate at higher levels**: Surface errors at meaningful boundaries (controllers, API layers) rather than propagating many small, specific errors through every layer.
- **Crash on true bugs**: For genuinely unrecoverable states or programmer errors, crash immediately rather than attempting fragile recovery.
- **Retry transient failures**: Use exponential backoff for transient failures in external service calls.

### Resource management

- **Clean up resources reliably**: Use RAII/Drop (Rust), finally blocks, or defer statements to ensure cleanup regardless of exit path. Don't rely on happy-path cleanup.
- **Flush and close gracefully**: Even with RAII, explicitly flush buffers and close connections when graceful shutdown semantics matter.

### User-facing errors

- **User-friendly messages**: Provide clear, actionable messages without exposing technical details.
- **Graceful degradation**: When non-critical services fail, degrade functionality rather than failing entirely.
