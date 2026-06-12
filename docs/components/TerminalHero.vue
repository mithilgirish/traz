<script setup>
import { ref, onMounted } from 'vue'

const typedText = ref('')
const fullText = 'The local-first AI memory layer_'
const isTyping = ref(true)

onMounted(() => {
  let i = 0
  const typeWriter = () => {
    if (i < fullText.length) {
      typedText.value += fullText.charAt(i)
      i++
      setTimeout(typeWriter, 50)
    } else {
      isTyping.value = false
    }
  }
  setTimeout(typeWriter, 500)
})
</script>

<template>
  <div class="terminal-hero">
    <div class="grid-bg"></div>
    <div class="glow-orb"></div>
    
    <div class="hero-content">
      
      
      <h1 class="main-title">Trace. Context. Continuity.</h1>
      
      <div class="typewriter-container">
        <h2 class="typewriter-text">{{ typedText }}<span class="cursor" :class="{ blink: !isTyping }">█</span></h2>
      </div>

      <p class="tagline">Give all your AI coding tools a shared brain. Capture decisions, track refactors, and never lose context again.</p>

      <div class="actions">
        <a href="/QUICKSTART" class="action-btn primary">Get Started</a>
        <a href="https://github.com/mithilgirish/traz" class="action-btn secondary">View on GitHub</a>
      </div>

      <div class="terminal-wrapper">
        <div class="terminal-window">
          <div class="terminal-header">
            <div class="mac-buttons">
              <span class="mac-btn close"></span>
              <span class="mac-btn minimize"></span>
              <span class="mac-btn maximize"></span>
            </div>
          </div>
          <div class="terminal-body">
            <div class="term-line"><span class="prompt">$</span> traz search "auth issue"</div>
            <div class="term-line output gray">[semantic search] Search: "auth issue" (2 results)</div>
            <div class="term-line output">─────────────────────────────────────────────</div>
            <div class="term-line output"> 1. Fixed JWT refresh race condition (89%)</div>
            <div class="term-line output gray">    Tool: cursor       Type: commit     Age: 2h ago</div>
            <br>
            <div class="term-line output"> 2. Added refresh token fallback (76%)</div>
            <div class="term-line output gray">    Tool: gemini       Type: refactor   Age: 1d ago</div>
            <div class="term-line mt"><span class="prompt">$</span> traz mcp start --agent claude</div>
            <div class="term-line output green">✔ MCP Server running. Connected to Claude Code.</div>
            <div class="term-line mt"><span class="prompt blink-term">█</span></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal-hero {
  position: relative;
  padding: 6rem 2rem 8rem 2rem;
  display: flex;
  justify-content: center;
  overflow: hidden;
  min-height: 80vh;
  background-color: var(--vp-c-bg); 
  color: var(--vp-c-text-1);
}

.grid-bg {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-image: 
    linear-gradient(to right, rgba(255, 255, 255, 0.03) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(255, 255, 255, 0.03) 1px, transparent 1px);
  background-size: 40px 40px;
  mask-image: radial-gradient(circle at top center, black, transparent 80%);
  -webkit-mask-image: radial-gradient(circle at top center, black, transparent 80%);
  z-index: 0;
  pointer-events: none;
}

.glow-orb {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 800px;
  height: 800px;
  background: radial-gradient(circle, rgba(99, 102, 241, 0.04) 0%, transparent 50%);
  z-index: 0;
  pointer-events: none;
}

.hero-content {
  position: relative;
  z-index: 1;
  max-width: 900px;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 14px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  font-family: var(--vp-font-family-mono), monospace;
  font-size: 0.8rem;
  margin-bottom: 2rem;
  color: #6366f1; /* Purple/Blue */
}

.status-dot {
  width: 6px;
  height: 6px;
  background-color: #6366f1;
  border-radius: 50%;
  box-shadow: 0 0 8px rgba(99, 102, 241, 0.4);
}

.main-title {
  font-family: var(--vp-font-family-mono), monospace;
  font-size: 4rem;
  font-weight: 800;
  line-height: 1.1;
  margin-bottom: 1rem;
  color: var(--vp-c-text-1);
  letter-spacing: -0.04em;
}

.typewriter-container {
  height: 3rem;
  margin-bottom: 1.5rem;
}

.typewriter-text {
  font-family: var(--vp-font-family-mono), monospace;
  font-size: 1.5rem;
  font-weight: 500;
  color: var(--vp-c-text-2); 
}

.cursor {
  opacity: 1;
  color: #6366f1;
}

.cursor.blink {
  animation: blink 1s step-end infinite;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.tagline {
  font-size: 1.25rem;
  color: var(--vp-c-text-2);
  max-width: 650px;
  line-height: 1.6;
  margin-bottom: 3rem;
}

.actions {
  display: flex;
  gap: 1rem;
  margin-bottom: 4rem;
}

.action-btn {
  padding: 0.75rem 1.75rem;
  border-radius: 6px;
  font-weight: 600;
  font-size: 1rem;
  transition: all 0.2s ease;
  text-decoration: none !important;
  font-family: var(--vp-font-family-mono), monospace;
}

.action-btn.primary {
  background-color: #6366f1;
  color: #ffffff;
}

.action-btn.primary:hover {
  background-color: #818cf8;
}

.action-btn.secondary {
  background-color: transparent;
  color: #6366f1;
  border: 1px solid rgba(99, 102, 241, 0.3);
}

.action-btn.secondary:hover {
  background-color: rgba(99, 102, 241, 0.05);
  border-color: rgba(99, 102, 241, 0.6);
}

.terminal-wrapper {
  width: 100%;
  display: flex;
  justify-content: center;
  position: relative;
  z-index: 2;
}

/* Minimalist Flat Terminal Window */
.terminal-window {
  width: 100%;
  max-width: 850px;
  background-color: #0d1117;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 20px 40px -10px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  text-align: left;
  transition: box-shadow 0.3s ease;
}

.terminal-window:hover {
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
}

.terminal-header {
  background-color: #161b22;
  padding: 0.75rem 1rem;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.mac-buttons {
  display: flex;
  gap: 6px;
}

.mac-btn {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.mac-btn.close { background-color: #ff5f56; }
.mac-btn.minimize { background-color: #ffbd2e; }
.mac-btn.maximize { background-color: #27c93f; }

.terminal-title {
  flex-grow: 1;
  text-align: center;
  font-family: var(--vp-font-family-mono), monospace;
  font-size: 0.8rem;
  color: rgba(255, 255, 255, 0.3);
  margin-right: 42px;
}

.terminal-body {
  padding: 1.5rem 2rem;
  font-family: 'Fira Code', var(--vp-font-family-mono), monospace;
  font-size: 0.9rem;
  line-height: 1.6;
  color: #c9d1d9;
}

.term-line {
  margin-bottom: 0.2rem;
}

.prompt {
  color: #6366f1;
  margin-right: 0.75rem;
  font-weight: bold;
}

.output {
  margin-left: 1.5rem;
}

.gray { color: #8b949e; }
.green { color: #818cf8; }
.mt { margin-top: 1rem; }

.blink-term {
  animation: blink 1s step-end infinite;
  color: #c9d1d9;
}

@media (max-width: 768px) {
  .main-title { font-size: 2.2rem; }
  .typewriter-text { font-size: 1.1rem; }
  .terminal-window { font-size: 0.8rem; }
}
</style>
