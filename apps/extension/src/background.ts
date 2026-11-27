import { toggleOverlay } from "./inject"

async function openReader(tab: chrome.tabs.Tab): Promise<void> {
  if (tab.id === undefined) return
  try {
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: toggleOverlay,
      args: [chrome.runtime.getURL("reader.html")],
    })
  } catch (error) {
    console.warn("clench: this page cannot be read", error)
    await chrome.action.setBadgeText({ tabId: tab.id, text: "!" })
    await chrome.action.setBadgeBackgroundColor({ tabId: tab.id, color: "#b3261e" })
    await chrome.action.setTitle({ tabId: tab.id, title: "clench cannot read this page (browser pages and the web store are off limits)" })
  }
}

chrome.action.onClicked.addListener((tab) => {
  void openReader(tab)
})

if (import.meta.env.MODE === "e2e") {
  Object.assign(globalThis, {
    clenchOpenActiveTab: async (): Promise<void> => {
      const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true })
      if (tab) await openReader(tab)
    },
  })
}
