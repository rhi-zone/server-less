import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(
  defineConfig({
    title: 'Server-less',
    description: 'Write less server code - composable derive macros for Rust',
    base: '/server-less/',

    themeConfig: {
      nav: [
        { text: 'Home', link: '/' },
        { text: 'Tutorials', link: '/tutorials/rest-api' },
      ],

      sidebar: {
        '/': [
          {
            text: 'Introduction',
            items: [
              { text: 'What is Server-less?', link: '/' },
            ]
          },
          {
            text: 'Design',
            items: [
              { text: 'Impl-First', link: '/design/impl-first' },
              { text: 'Extension Coordination', link: '/design/extension-coordination' },
              { text: 'Implementation Notes', link: '/design/implementation-notes' },
            ]
          },
          {
            text: 'Tutorials',
            items: [
              { text: 'REST API', link: '/tutorials/rest-api' },
              { text: 'Multi-Protocol', link: '/tutorials/multi-protocol' },
            ]
          },
        ]
      },

      socialLinks: [
        { icon: 'github', link: 'https://github.com/rhi-zone/server-less' }
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
