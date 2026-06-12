import DefaultTheme from 'vitepress/theme'
import './style.css'
import { h } from 'vue'
import TerminalHero from '../../components/TerminalHero.vue'

export default {
  extends: DefaultTheme,
  Layout() {
    return h(DefaultTheme.Layout, null, {
      'home-hero-before': () => h(TerminalHero)
    })
  }
}
