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
              { text: 'Overview', link: '/design/' },
              { text: 'Impl-First', link: '/design/impl-first' },
              { text: 'Inference vs Configuration', link: '/design/inference-vs-configuration' },
              { text: 'Param Attributes', link: '/design/param-attributes' },
              { text: 'Error Mapping', link: '/design/error-mapping' },
              { text: 'Extension Coordination', link: '/design/extension-coordination' },
              { text: 'Parse-Time Coordination', link: '/design/parse-time-coordination' },
              { text: 'Protocol Naming', link: '/design/protocol-naming' },
              { text: 'Blessed Presets', link: '/design/blessed-presets' },
              { text: 'CLI Output Formatting', link: '/design/cli-output-formatting' },
              { text: 'Route & Response Attrs', link: '/design/route-response-attrs' },
              { text: 'Mount Points', link: '/design/mount-points' },
              { text: 'OpenAPI Composition', link: '/design/openapi-composition' },
              { text: 'Open Questions', link: '/design/open-questions' },
              { text: 'Iteration Log', link: '/design/iteration-log' },
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
