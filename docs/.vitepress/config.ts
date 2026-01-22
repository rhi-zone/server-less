import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(
  defineConfig({
    title: 'Trellis',
    description: 'Composable derive macros for Rust',

    themeConfig: {
      nav: [
        { text: 'Home', link: '/' },
        { text: 'Guide', link: '/guide/' },
      ],

      sidebar: {
        '/': [
          {
            text: 'Introduction',
            items: [
              { text: 'What is Trellis?', link: '/' },
              { text: 'Getting Started', link: '/guide/' },
            ]
          },
          {
            text: 'Macros',
            items: [
              { text: 'Overview', link: '/macros/' },
            ]
          },
        ]
      },

      socialLinks: [
        { icon: 'github', link: 'https://github.com/rhizome-lab/server-less' }
      ],

      search: {
        provider: 'local'
      },
    },

    vite: {
      optimizeDeps: {
        include: ['mermaid'],
      },
    },
  }),
)
