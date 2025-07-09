## Pipeline architecture

It should work like this


- PipelineMetadataStash (one per request)

- Pipeline
    - ::handle_request
    - ::handle_response
    - plugins
    
- Directory ISA Pipeline
    - ::handle_request ( returns Response or None )
        - note, only executes the plugins when needed
    - ::handle_response
    - plugins