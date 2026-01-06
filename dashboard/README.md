# Piramid Dashboard

A React dashboard for managing your Piramid vector database.

## Structure

```
dashboard/
├── app/
│   ├── page.tsx              # Main dashboard (orchestration only)
│   ├── layout.tsx            # Root layout with theme
│   ├── globals.css           # CSS variables & dark theme
│   ├── lib/
│   │   └── api.ts            # API client - all server communication
│   └── components/
│       ├── Sidebar.tsx       # Collection list & navigation
│       ├── OverviewTab.tsx   # Stats & quick insert
│       ├── SearchTab.tsx     # Vector search playground
│       ├── BrowseTab.tsx     # Vector list & management
│       ├── Modal.tsx         # Create collection dialog
│       └── ServerOffline.tsx # Offline state UI
├── package.json
├── next.config.ts
└── tailwind.config.ts
```

## Running

```bash
# Install dependencies
npm install

# Development server (port 3000)
npm run dev

# Build for production (static export)
npm run build
```

## Building for Production

The dashboard builds to static HTML/JS/CSS that can be served by the Python server:

```bash
npm run build
# Output goes to ./out/
```

The server will serve these files at `http://localhost:6333/`

## Customization

Theme colors are defined as CSS variables in `globals.css`:

```css
:root {
  --bg-primary: #0f0f0f;
  --bg-secondary: #1a1a1a;
  --accent: #8b5cf6;
  --success: #10b981;
  --error: #ef4444;
}
```

## API Client

All server communication goes through `lib/api.ts`:

```typescript
import { listCollections, searchVectors } from './lib/api';

// List collections
const collections = await listCollections();

// Search
const results = await searchVectors('my-collection', {
  vector: [0.1, 0.2, 0.3],
  limit: 10,
  metric: 'cosine',
});
```
