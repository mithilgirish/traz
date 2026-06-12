import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "traz",
  titleTemplate: false,
  description: "The local-first developer memory layer that gives AI coding tools a shared brain.",
  cleanUrls: true,
  
  head: [
    ['link', { rel: 'icon', href: '/favicon.png' }],
    ['meta', { name: 'theme-color', content: '#6366f1' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:locale', content: 'en' }],
    ['meta', { property: 'og:title', content: 'traz | AI Context Memory Layer' }],
    ['meta', { property: 'og:site_name', content: 'traz' }],
    ['meta', { property: 'og:image', content: 'https://traz.mithilgirish.dev/logo.png' }],
    ['meta', { property: 'og:url', content: 'https://traz.mithilgirish.dev/' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:image', content: 'https://traz.mithilgirish.dev/logo.png' }],
    ['meta', { name: 'robots', content: 'index, follow' }],
    // Optimize for AI agents browsing the docs
    ['link', { rel: 'alternate', type: 'text/plain', title: 'LLM-friendly text', href: '/llms.txt' }]
  ],

  sitemap: {
    hostname: 'https://traz.mithilgirish.dev'
  },

  themeConfig: {
    logo: '/logo.png',
    
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Quickstart', link: '/QUICKSTART' },
      { text: 'User Guide', link: '/USER_GUIDE' },
      {
        text: 'v0.1.0',
        items: [
          { text: 'v0.1.0 (Current)', link: '/' },
          { text: 'Changelog', link: 'https://github.com/mithilgirish/traz/blob/main/CHANGELOG.md' }
        ]
      }
    ],

    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Quickstart', link: '/QUICKSTART' },
          { text: 'User Guide', link: '/USER_GUIDE' }
        ]
      },
      {
        text: 'Integration',
        items: [
          { text: 'MCP Integration', link: '/MCP_INTEGRATION' },
          { text: 'Agent Integration', link: '/AGENT_INTEGRATION' }
        ]
      },
      {
        text: 'Under the Hood',
        items: [
          { text: 'Architecture', link: '/ARCHITECTURE' }
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/mithilgirish/traz' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026-present'
    },
    
    search: {
      provider: 'local'
    }
  }
})
