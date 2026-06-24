# ocotelolco

## Website

Generate the website locally, including its adjacent `assets/` directory:

```sh
cargo run -- make-website
```

Generate `docs/index.html`, commit only the GitHub Pages output, and push it:

```sh
cargo run -- deploy-website
```

Deployment must be run from `main` with push access to the `origin` remote.
