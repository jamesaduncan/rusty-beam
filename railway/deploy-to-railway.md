# Deploy to Railway

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https%3A%2F%2Fgithub.com%2Fjamesaduncan%2Frusty-beam&plugins=&envs=DOCS_GIT_REPO%2CADDITIONAL_HOSTNAMES&DOCS_GIT_REPODesc=Git+repository+URL+for+documentation&ADDITIONAL_HOSTNAMESDesc=Comma-separated+list+of+additional+hostnames)

Or deploy manually:

1. Create a new project on Railway
2. Choose "Deploy from Docker Hub"
3. Enter image: `jamesaduncan774/rusty-beam:latest`
4. Set environment variables:
   - `DOCS_GIT_REPO` - Your documentation git repository
   - `ADDITIONAL_HOSTNAMES` - Additional hostnames (optional)