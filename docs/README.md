# Q Documentation Site

This folder contains the documentation website for Q - Filesystem & Process SQL Query Tool.

## Deployment

The documentation site is configured for GitHub Pages deployment from the `docs/` folder.

### Automatic Deployment
1. Push changes to the `main` branch
2. Go to Repository Settings > Pages
3. Set Source to "Deploy from a branch"
4. Set Branch to "main" and folder to "/docs"
5. The site will be available at `https://willdech.github.io/query-os/`

### Local Development
To test the site locally:
```bash
cd docs
python3 -m http.server 8000
# Visit http://localhost:8000
```

## Structure
- `index.html` - Main documentation page
- `.nojekyll` - Disables Jekyll processing for plain HTML/CSS/JS

## Technologies Used
- HTML5
- Tailwind CSS (via CDN)
- Vanilla JavaScript
- Prism.js for syntax highlighting
- Font Awesome for icons

## Customization
- Colors can be modified in the Tailwind config within `index.html`
- Content sections are clearly marked and can be easily updated
- All examples are in code blocks with proper syntax highlighting
