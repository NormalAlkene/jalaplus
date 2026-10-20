// ==UserScript==
// @name         JaLaPlus
// @namespace    https://github.com/NormalAlkene/jalaplus
// @version      0.1.0
// @description  保存 jala.tools JLPT 题目的作答、答案与解析，导出 JSON
// @include      /^https:\/\/jala\.tools\/[a-z]{2}\/jlpt(?:\/|$)/
// @run-at       document-idle
// @grant        none
// @noframes
// ==/UserScript==

(() => {
  'use strict';
  if (window.__jalaplusInstalled) return;
  window.__jalaplusInstalled = true;

  const VERSION = '0.1.0';
  const STORAGE_KEY = 'jalaplus:v1';
  const HOST_ID = 'jalaplus-root';

  const CATEGORY_LABELS = {
    characters: '文字',
    vocabulary: '词汇',
    grammar: '语法',
    reading: '阅读',
    listening: '听力',
  };

  const CHAPTER_LABELS = {
    'kanji-reading': '汉字读法',
    orthography: '汉字书写',
    'word-formation': '构词',
    'contextually-defined': '语境词义',
    paraphrase: '近义替换',
    usage: '用法',
    'grammar-form': '语法形式判断',
    'sentence-composition': '句子组合',
    'text-grammar': '文章语法',
    'short-passage': '内容理解（短篇）',
    'medium-passage': '内容理解（中篇）',
    'integrated-comparison': '综合理解',
    'thematic-passage': '主张理解（长篇）',
    'information-retrieval': '信息检索',
    'task-based-comprehension': '任务理解',
    'key-points': '重点理解',
    'general-outline': '概要理解',
    'quick-response': '即时应答',
    'integrated-comprehension': '综合理解',
  };

  const CATEGORY_ORDER = {
    characters: 0,
    vocabulary: 1,
    grammar: 2,
    reading: 3,
    listening: 4,
  };

  const CHAPTER_ORDER = {
    'kanji-reading': 0,
    orthography: 1,
    'word-formation': 2,
    'contextually-defined': 3,
    paraphrase: 4,
    usage: 5,
    'grammar-form': 6,
    'sentence-composition': 7,
    'text-grammar': 8,
    'short-passage': 9,
    'medium-passage': 10,
    'integrated-comparison': 11,
    'thematic-passage': 12,
    'information-retrieval': 13,
    'task-based-comprehension': 14,
    'key-points': 15,
    'general-outline': 16,
    'quick-response': 17,
    'integrated-comprehension': 18,
  };

  const ROUTE_RE = /^\/([a-z]{2})\/jlpt(?:\/(n[1-5])(?:\/(\d{4}-\d{2})(?:\/([^/]+)\/([^/]+)\/(\d+))?)?)?\/?$/;

  const state = {
    route: null,
    modalKind: null,
    view: 'paper',
    selected: new Set(),
    expandedGroups: new Set(),
    expandedQuestions: new Set(),
    storageTab: 'local',
    expandedKeys: new Set(),
    progress: null,
    batchRunning: false,
    batchCancel: false,
    activeConfirm: null,
  };

  let shell = null;

  // ---------- 基础工具 ----------

  function uuid() {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID();
    }
    return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
  }

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function h(tag, attrs = {}, ...children) {
    const node = document.createElement(tag);
    for (const [key, value] of Object.entries(attrs || {})) {
      if (value === null || value === undefined || value === false) continue;
      if (key === 'class') node.className = value;
      else if (key === 'style') node.style.cssText = value;
      else if (key === 'dataset') Object.assign(node.dataset, value);
      else if (key.startsWith('on') && typeof value === 'function') {
        node.addEventListener(key.slice(2).toLowerCase(), value);
      } else if (value === true) node.setAttribute(key, '');
      else node.setAttribute(key, String(value));
    }
    for (const child of children.flat(Infinity)) {
      if (child === null || child === undefined || child === false) continue;
      node.append(child instanceof Node ? child : document.createTextNode(String(child)));
    }
    return node;
  }

  function icon(name) {
    const paths = {
      plus: '<path d="M12 5v14M5 12h14"/>',
      collection: '<path d="M6 3h12a1 1 0 0 1 1 1v17l-7-4-7 4V4a1 1 0 0 1 1-1z"/>',
      storage: '<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5"/><path d="M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/>',
      close: '<path d="M18 6 6 18M6 6l12 12"/>',
      chevron: '<path d="m9 18 6-6-6-6"/>',
      download: '<path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/>',
      trash: '<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/>',
      refresh: '<path d="M21 12a9 9 0 1 1-2.6-6.4"/><path d="M21 3v6h-6"/>',
      record: '<path d="M4 5h16M4 12h16M4 19h10"/>',
    };
    return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] || ''}</svg>`;
  }

  function iconNode(name) {
    const span = document.createElement('span');
    span.className = 'jp-icon-svg';
    span.innerHTML = icon(name);
    return span;
  }

  function formatDate(iso) {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return iso || '';
    const pad = (value) => String(value).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
  }

  function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  }

  function textSize(text) {
    return new TextEncoder().encode(text).length;
  }

  function toast(message, kind = 'info') {
    if (!shell) return;
    const node = h('div', { class: `jp-toast ${kind}` }, message);
    shell.toastWrap.append(node);
    setTimeout(() => node.remove(), kind === 'error' ? 5200 : 3400);
  }

  // ---------- 路由与页面信息 ----------

  function parseRoute(path = location.pathname) {
    const match = ROUTE_RE.exec(path);
    if (!match) return null;
    return {
      locale: match[1],
      level: match[2] || null,
      exam: match[3] || null,
      category: match[4] || null,
      chapter: match[5] || null,
      index: match[6] ? Number(match[6]) : null,
    };
  }

  function routeFromHref(href) {
    try {
      return parseRoute(new URL(href, location.origin).pathname);
    } catch {
      return null;
    }
  }

  function routeKey(route) {
    if (!route || !route.level || !route.exam) return null;
    return `${route.level}/${route.exam}`;
  }

  function questionKey(record) {
    const q = record.question || {};
    return `${record.paper.level}/${record.paper.exam}/${q.category}/${q.chapter}/${q.index}`;
  }

  function paperKey(record) {
    return `${record.paper.level}/${record.paper.exam}`;
  }

  function typeKey(record) {
    const q = record.question || {};
    return `${q.category}/${q.chapter}`;
  }

  function paperTitle(record) {
    return record.paper.title || `JLPT ${String(record.paper.level).toUpperCase()}・${record.paper.exam}`;
  }

  function currentPaperKey() {
    return routeKey(state.route);
  }

  function readBreadcrumb() {
    const nav = document.querySelector('nav[aria-label="Breadcrumb"]');
    if (!nav) return [];
    return [...nav.querySelectorAll(':scope > span')]
      .map((span) => span.textContent.replace(/›/g, '').trim())
      .filter(Boolean);
  }

  function readPageLabels() {
    const items = readBreadcrumb();
    const route = state.route;
    const categoryLabel = items.length >= 5 ? items[3] : null;
    const chapterLabel = items.length >= 6 ? items[4] : null;
    return {
      category: categoryLabel || (route && CATEGORY_LABELS[route.category]) || (route ? route.category : ''),
      chapter: chapterLabel || (route && CHAPTER_LABELS[route.chapter]) || (route ? route.chapter : ''),
    };
  }

  function readPaperTitle(route) {
    if (!route || !route.level || !route.exam) return '';
    const items = readBreadcrumb();
    const examLabel = items.length >= 3 ? items[2] : '';
    return `JLPT ${String(route.level).toUpperCase()}・${examLabel || route.exam}`;
  }

  function readExamPage() {
    const title = (document.querySelector('h1')?.textContent || '').trim();
    const pages = [];
    for (const section of document.querySelectorAll('section')) {
      for (const anchor of section.querySelectorAll('a.block[href*="/jlpt/"]')) {
        const href = anchor.getAttribute('href');
        const route = routeFromHref(href);
        if (!route || !route.level || !route.exam || !route.category || !route.chapter) continue;
        const count = Number((anchor.querySelector('.t-caption')?.textContent || '').trim());
        if (!Number.isFinite(count) || count < 1) continue;
        const chapterLabel = (anchor.querySelector('.t-label')?.textContent || '').trim();
        const base = href.replace(/\/\d+$/, '');
        for (let index = 1; index <= count; index += 1) {
          pages.push({
            href: `${base}/${index}`,
            route: { ...route, index },
            categoryLabel: CATEGORY_LABELS[route.category] || route.category,
            chapterLabel: chapterLabel || CHAPTER_LABELS[route.chapter] || route.chapter,
          });
        }
      }
    }
    return pages.length ? { title, pages } : null;
  }

  // ---------- 页面数据 ----------

  function sliceBalanced(text, start) {
    if (start < 0 || text[start] !== '{') return null;
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (let i = start; i < text.length; i += 1) {
      const char = text[i];
      if (inString) {
        if (escaped) escaped = false;
        else if (char === '\\') escaped = true;
        else if (char === '"') inString = false;
      } else if (char === '"') {
        inString = true;
      } else if (char === '{') {
        depth += 1;
      } else if (char === '}') {
        depth -= 1;
        if (depth === 0) return text.slice(start, i + 1);
      }
    }
    return null;
  }

  function cleanUndefined(value) {
    if (value === '$undefined') return null;
    if (Array.isArray(value)) return value.map(cleanUndefined);
    if (value && typeof value === 'object') {
      const result = {};
      for (const [key, item] of Object.entries(value)) result[key] = cleanUndefined(item);
      return result;
    }
    return value;
  }

  function flightTextFromNode(root) {
    let text = '';
    for (const script of root.querySelectorAll('script')) {
      const source = script.textContent || '';
      if (!source.includes('__next_f.push')) continue;
      const pattern = /self\.__next_f\.push\(\[1,("(?:\\.|[^"\\])*")\]\)/g;
      let match;
      while ((match = pattern.exec(source))) {
        try {
          text += JSON.parse(match[1]);
        } catch {
          // 跳过无法解析的分片，页面数据通常仍有其他完整分片
        }
      }
    }
    return text;
  }

  function flightTextFromPage() {
    let text = '';
    if (Array.isArray(window.__next_f)) {
      for (const chunk of window.__next_f) {
        if (Array.isArray(chunk) && typeof chunk[1] === 'string') text += chunk[1];
      }
    }
    return text + flightTextFromNode(document);
  }

  function extractQuestionData(text, route) {
    if (!text || !route || !route.category) return null;
    const matches = (data) =>
      data &&
      data.level === route.level &&
      data.exam === route.exam &&
      data.category === route.category &&
      data.chapter === route.chapter &&
      Number(data.index) === Number(route.index) &&
      Array.isArray(data.groups);

    const marker = `"data":{"level":"${route.level}","exam":"${route.exam}","category":"${route.category}","chapter":"${route.chapter}","index":${route.index}`;
    let position = text.indexOf(marker);
    if (position >= 0) {
      const brace = text.indexOf('{', position);
      const raw = sliceBalanced(text, brace);
      if (raw) {
        try {
          const data = cleanUndefined(JSON.parse(raw));
          if (matches(data)) return data;
        } catch {
          // 继续扫描其他 data 对象
        }
      }
    }

    position = 0;
    while ((position = text.indexOf('"data":', position)) >= 0) {
      const brace = text.indexOf('{', position);
      const raw = sliceBalanced(text, brace);
      if (raw) {
        try {
          const data = cleanUndefined(JSON.parse(raw));
          if (matches(data)) return data;
        } catch {
          // 忽略不完整或非题目对象
        }
      }
      position += 7;
    }
    return null;
  }

  async function loadQuestionData(route) {
    const fromPage = extractQuestionData(flightTextFromPage(), route);
    if (fromPage) return fromPage;

    const response = await fetch(location.href, {
      credentials: 'same-origin',
      headers: { Accept: 'text/html' },
    });
    if (!response.ok) throw new Error(`读取页面失败（HTTP ${response.status}）`);
    const html = await response.text();
    const documentCopy = new DOMParser().parseFromString(html, 'text/html');
    const fromCopy = extractQuestionData(flightTextFromNode(documentCopy), route);
    if (fromCopy) return fromCopy;
    throw new Error('页面中没有找到题目数据，请刷新后重试');
  }

  function localize(value, locale) {
    if (typeof value === 'string') return value.trim() || null;
    if (!value || typeof value !== 'object') return null;
    for (const key of [locale, 'zh', 'en', 'ja', 'ko', 'vi']) {
      if (typeof value[key] === 'string' && value[key].trim()) return value[key].trim();
    }
    const first = Object.values(value).find((item) => typeof item === 'string' && item.trim());
    return first ? first.trim() : null;
  }

  function isPlaceholder(text) {
    return typeof text === 'string' && /lorem ipsum/i.test(text);
  }

  function cleanTranscript(transcript, locale) {
    if (!transcript || typeof transcript !== 'object') {
      return { transcript: null, access: 'absent' };
    }
    const sentences = Array.isArray(transcript.sentences)
      ? transcript.sentences
          .filter((sentence) => sentence && typeof sentence.text === 'string')
          .map((sentence) => ({
            startMs: Number(sentence.startMs) || 0,
            endMs: Number(sentence.endMs) || 0,
            roleId: Number.isInteger(sentence.roleId) ? sentence.roleId : -1,
            text: sentence.text,
          }))
      : [];
    if (!sentences.length) return { transcript: null, access: 'absent' };
    const text = sentences.map((sentence) => sentence.text).join(' ');
    if (isPlaceholder(text)) return { transcript: null, access: 'locked' };
    return {
      transcript: {
        roles: Array.isArray(transcript.roles)
          ? transcript.roles.map((role) => ({
              roleId: Number(role.roleId) || 0,
              name: String(role.name || ''),
            }))
          : [],
        sentences,
        locale,
      },
      access: 'full',
    };
  }

  function buildQuestion(data, route, locale) {
    const groups = (data.groups || [])
      .filter((group) => group && Array.isArray(group.options) && group.options.length)
      .map((group, index) => ({
        id: String(group.id || `g${index + 1}`),
        prompt: String(group.prompt || ''),
        options: group.options
          .filter((option) => option && option.id !== null && option.id !== undefined)
          .map((option) => ({
            id: String(option.id),
            label: String(option.label ?? ''),
          })),
        answer: group.answer === null || group.answer === undefined ? null : String(group.answer),
        explanation: null,
        audio: group.audio ? String(group.audio) : null,
      }));

    const explanationCandidates = [];
    const topExplanationRaw = localize(data.explanation, locale);
    if (topExplanationRaw) explanationCandidates.push(topExplanationRaw);
    for (const [index, group] of (data.groups || []).entries()) {
      const groupExplanation = localize(group && group.explanation, locale);
      if (groupExplanation) {
        explanationCandidates.push(groupExplanation);
        if (groups[index] && !isPlaceholder(groupExplanation)) groups[index].explanation = groupExplanation;
      }
    }

    let explanation = null;
    let explanationAccess = 'absent';
    const realExplanation = explanationCandidates.find((text) => !isPlaceholder(text));
    if (realExplanation) {
      explanation = topExplanationRaw && !isPlaceholder(topExplanationRaw) ? topExplanationRaw : realExplanation;
      explanationAccess = 'full';
    } else if (explanationCandidates.length) {
      explanationAccess = 'locked';
    }

    const transcriptResult = cleanTranscript(data.transcript, locale);
    const passage = data.passage && typeof data.passage.text === 'string'
      ? {
          text: data.passage.text,
          translation: localize(data.passage.tr, locale),
        }
      : null;

    return {
      question: {
        category: route.category,
        chapter: route.chapter,
        index: Number(route.index),
        prompt: String(data.prompt || ''),
        passage,
        audio: data.audio ? String(data.audio) : null,
        optionsHidden: Boolean(data.optionsHidden),
        groups,
        transcript: transcriptResult.transcript,
        explanation,
      },
      access: {
        explanation: explanationAccess,
        transcript: transcriptResult.access,
      },
    };
  }

  function readAnswers(groups) {
    const groupElements = [...document.querySelectorAll('[data-sentry-component="OptionGroup"]')];
    const answers = {};
    const count = Math.min(groupElements.length, groups.length);
    for (let index = 0; index < count; index += 1) {
      const buttons = [...groupElements[index].querySelectorAll('button[aria-pressed]')];
      const selected = buttons.findIndex((button) => button.getAttribute('aria-pressed') === 'true');
      if (selected < 0) continue;
      const group = groups[index];
      answers[group.id] = group.options[selected]?.id || String.fromCharCode(97 + selected);
    }
    return Object.keys(answers).length ? answers : null;
  }

  function readChecked() {
    const groupElements = [...document.querySelectorAll('[data-sentry-component="OptionGroup"]')];
    return groupElements.some((group) =>
      [...group.querySelectorAll('button')].some((button) => button.disabled),
    );
  }

  function readSiteStats(route) {
    if (!route || !route.category) return null;
    const key = `jlpt:stats:${route.level}/${route.exam}/${route.category}/${route.chapter}/${route.index}`;
    try {
      const raw = localStorage.getItem(key);
      if (!raw) return null;
      const value = JSON.parse(raw);
      const correct = Number(value.correct);
      const wrong = Number(value.wrong);
      if (!Number.isFinite(correct) || !Number.isFinite(wrong)) return null;
      return { correct: Math.max(0, Math.trunc(correct)), wrong: Math.max(0, Math.trunc(wrong)) };
    } catch {
      return null;
    }
  }

  function buildRecord({ route, data, source, batchId, labels, title, answers, checked, siteStats, url, paperMeta }) {
    const locale = route.locale;
    const built = buildQuestion(data, route, locale);
    const questionLabels = {
      category: labels?.category || CATEGORY_LABELS[route.category] || route.category,
      chapter: labels?.chapter || CHAPTER_LABELS[route.chapter] || route.chapter,
    };
    const totalGroups = built.question.groups.length;
    return {
      id: uuid(),
      capturedAt: new Date().toISOString(),
      source,
      batchId: batchId || null,
      paper: {
        level: route.level,
        exam: route.exam,
        title: title || readPaperTitle(route) || `JLPT ${String(route.level).toUpperCase()}・${route.exam}`,
        locale,
        pageCount: paperMeta?.pageCount ?? null,
        chapterPages: paperMeta?.chapterPages ?? null,
        groupCount: paperMeta?.groupCount ?? null,
        chapterGroups: paperMeta?.chapterGroups ?? null,
        complete: paperMeta?.complete ?? null,
      },
      question: {
        ...built.question,
        url: url || `${location.origin}${location.pathname}`,
        labels: questionLabels,
      },
      attempt: {
        answers: answers || null,
        answeredGroups: answers ? Object.keys(answers).length : 0,
        totalGroups,
        checked: Boolean(checked),
        siteStats: siteStats || null,
      },
      access: built.access,
    };
  }

  // ---------- 收藏集存储 ----------

  function loadStore() {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { version: 1, records: [] };
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch {
      throw new Error('收藏集数据损坏，无法读取');
    }
    if (!parsed || parsed.version !== 1 || !Array.isArray(parsed.records)) {
      throw new Error('收藏集数据格式不正确');
    }
    return parsed;
  }

  function saveStore(store) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
    } catch (error) {
      if (error && error.name === 'QuotaExceededError') {
        throw new Error('浏览器存储空间不足，请先删除部分记录');
      }
      throw error;
    }
  }

  function storeSize() {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? textSize(raw) : 0;
  }

  function addRecord(record) {
    const store = loadStore();
    store.records.push(record);
    saveStore(store);
  }

  function sortRecordsForExport(records) {
    return [...records].sort((a, b) => {
      const paper = paperKey(a).localeCompare(paperKey(b));
      if (paper) return paper;
      const category = (CATEGORY_ORDER[a.question.category] ?? 99) - (CATEGORY_ORDER[b.question.category] ?? 99);
      if (category) return category;
      const chapter = (CHAPTER_ORDER[a.question.chapter] ?? 99) - (CHAPTER_ORDER[b.question.chapter] ?? 99);
      if (chapter) return chapter;
      const index = Number(a.question.index) - Number(b.question.index);
      if (index) return index;
      return String(a.capturedAt).localeCompare(String(b.capturedAt));
    });
  }

  function exportFile(records, kind, description, slug) {
    const envelope = {
      format: 'jalaplus.export',
      version: 1,
      generator: `JaLaPlus ${VERSION}`,
      exportedAt: new Date().toISOString(),
      scope: { kind, description },
      records: sortRecordsForExport(records),
    };
    const date = new Date().toISOString().slice(0, 10);
    const blob = new Blob([JSON.stringify(envelope, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `jalaplus-${slug || 'export'}-${date}.json`;
    link.style.display = 'none';
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 2000);
    toast(`已导出 ${records.length} 条记录`, 'success');
  }

  // ---------- 保存单题与抓取试卷 ----------

  async function captureQuestion() {
    const route = parseRoute();
    if (!route || !route.category || !route.index) {
      toast('请先打开一道题目', 'info');
      return;
    }
    try {
      const data = await loadQuestionData(route);
      const built = buildQuestion(data, route, route.locale);
      const answers = readAnswers(built.question.groups);
      const checked = readChecked();
      const record = buildRecord({
        route,
        data,
        source: 'question',
        batchId: null,
        labels: readPageLabels(),
        title: readPaperTitle(route),
        answers,
        checked,
        siteStats: readSiteStats(route),
        url: `${location.origin}${location.pathname}`,
      });
      addRecord(record);
      toast(
        answers
          ? '已加入收藏集'
          : '当前未作答，已保存题目和答案，未记录作答',
        answers ? 'success' : 'info',
      );
    } catch (error) {
      toast(`保存失败：${error.message}`, 'error');
    }
  }

  async function capturePaper() {
    const route = parseRoute();
    if (!route || !route.level || !route.exam || route.category) {
      toast('请先打开一套试卷的首页', 'info');
      return;
    }
    if (state.batchRunning) return;
    const exam = readExamPage();
    if (!exam) {
      toast('没有在页面上找到题目列表', 'error');
      return;
    }
    const confirmed = await askConfirm(
      `抓取整套试卷？共 ${exam.pages.length} 个页面，需要逐页读取，大约 1 分钟。`,
      '开始抓取',
      false,
    );
    if (!confirmed) return;

    const chapterPages = {};
    for (const page of exam.pages) {
      const key = `${page.route.category}/${page.route.chapter}`;
      chapterPages[key] = (chapterPages[key] || 0) + 1;
    }
    const paperMeta = {
      pageCount: exam.pages.length,
      chapterPages,
      groupCount: null,
      chapterGroups: null,
      complete: null,
    };
    const batchId = uuid();
    state.batchRunning = true;
    state.batchCancel = false;
    state.progress = { title: '抓取整套试卷', total: exam.pages.length, done: 0, ok: 0, failed: 0, cancelled: false };
    state.modalKind = 'progress';
    renderModalRoot({ preserveScroll: false });

    let ok = 0;
    let failed = 0;
    let lastError = '';
    let fatalStop = false;
    try {
      for (const [index, page] of exam.pages.entries()) {
        if (state.batchCancel) break;
        try {
          const response = await fetch(page.href, {
            credentials: 'same-origin',
            headers: { Accept: 'text/html' },
          });
          if (response.status === 403) {
            throw new Error('网站要求人机验证，请先在浏览器中打开该页面再重试');
          }
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          const html = await response.text();
          const copy = new DOMParser().parseFromString(html, 'text/html');
          const data = extractQuestionData(flightTextFromNode(copy), page.route);
          if (!data) throw new Error('未找到题目数据');
          const record = buildRecord({
            route: page.route,
            data,
            source: 'exam',
            batchId,
            labels: { category: page.categoryLabel, chapter: page.chapterLabel },
            title: exam.title,
            answers: null,
            checked: false,
            siteStats: readSiteStats(page.route),
            url: `${location.origin}${new URL(page.href, location.origin).pathname}`,
            paperMeta,
          });
          addRecord(record);
          ok += 1;
        } catch (error) {
          failed += 1;
          lastError = error.message;
          if (/存储空间不足/.test(error.message)) {
            fatalStop = true;
          }
        }
        state.progress.done = index + 1;
        state.progress.ok = ok;
        state.progress.failed = failed;
        state.progress.cancelled = state.batchCancel;
        renderModalRoot({ preserveScroll: false });
        if (fatalStop) break;
        await sleep(180);
      }
    } finally {
      try {
        finalizeBatch(batchId, exam.pages.length, state.batchCancel || fatalStop);
      } catch {
        // 元数据写入失败时保留已保存的记录
      }
      state.batchRunning = false;
      state.progress = null;
      state.modalKind = null;
      renderModalRoot({ preserveScroll: false });
    }

    if (state.batchCancel) {
      toast(`已取消，保存了 ${ok} 条记录`, 'info');
    } else if (failed && !ok) {
      toast(`抓取失败：${lastError || '没有可用记录'}`, 'error');
    } else if (failed) {
      toast(`抓取完成：成功 ${ok} 条，失败 ${failed} 条。${lastError}`, 'info');
    } else {
      toast(`抓取完成：成功 ${ok} 条记录`, 'success');
    }
  }

  function finalizeBatch(batchId, pageCount, cancelled) {
    const store = loadStore();
    const batchRecords = store.records.filter((record) => record.batchId === batchId);
    if (!batchRecords.length) return;
    const chapterGroups = {};
    let groupCount = 0;
    for (const record of batchRecords) {
      const key = `${record.question.category}/${record.question.chapter}`;
      const count = record.question.groups.length;
      groupCount += count;
      chapterGroups[key] = (chapterGroups[key] || 0) + count;
    }
    const complete = !cancelled && batchRecords.length === pageCount;
    for (const record of batchRecords) {
      record.paper.groupCount = groupCount;
      record.paper.chapterGroups = chapterGroups;
      record.paper.complete = complete;
    }
    saveStore(store);
  }

  // ---------- 收藏集界面 ----------

  function groupQuestions(records) {
    const groups = new Map();
    for (const record of records) {
      const key = questionKey(record);
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(record);
    }
    for (const list of groups.values()) {
      list.sort((a, b) => String(b.capturedAt).localeCompare(String(a.capturedAt)));
    }
    return groups;
  }

  function compareQuestionKeys(a, b, groups) {
    const left = groups.get(a)[0];
    const right = groups.get(b)[0];
    const paper = paperKey(left).localeCompare(paperKey(right));
    if (paper) return paper;
    const category = (CATEGORY_ORDER[left.question.category] ?? 99) - (CATEGORY_ORDER[right.question.category] ?? 99);
    if (category) return category;
    const chapter = (CHAPTER_ORDER[left.question.chapter] ?? 99) - (CHAPTER_ORDER[right.question.chapter] ?? 99);
    if (chapter) return chapter;
    return Number(left.question.index) - Number(right.question.index);
  }

  function answerText(group, answerId) {
    if (!answerId) return '';
    const index = group.options.findIndex((option) => option.id === answerId);
    if (index < 0) return answerId;
    const option = group.options[index];
    const label = /\.(jpe?g|png|gif|webp)(\?|$)/i.test(option.label) ? '[图片]' : option.label;
    return `${index + 1} ${label}`;
  }

  function attemptSummary(record) {
    const groups = record.question.groups || [];
    const answers = record.attempt.answers || null;
    if (!answers || !Object.keys(answers).length) {
      return {
        label: '未作答',
        cls: 'warn',
        detail: record.attempt.siteStats
          ? `网站计数：正确 ${record.attempt.siteStats.correct}，错误 ${record.attempt.siteStats.wrong}`
          : '没有记录选择结果',
      };
    }
    let correct = 0;
    let wrong = 0;
    const parts = [];
    for (const [index, group] of groups.entries()) {
      const selected = answers[group.id];
      if (!selected) continue;
      const user = answerText(group, selected);
      const answer = answerText(group, group.answer);
      const isCorrect = group.answer && selected === group.answer;
      if (isCorrect) correct += 1;
      else wrong += 1;
      const prefix = groups.length > 1 ? `第${index + 1}小题：` : '';
      parts.push(`${prefix}你选 ${user}${isCorrect ? '（正确）' : `（正确为 ${answer}）`}`);
    }
    const label = wrong === 0 && correct === groups.length ? '正确' : wrong === 0 ? '部分作答' : '错误';
    return {
      label,
      cls: wrong === 0 && correct === groups.length ? 'ok' : wrong === 0 ? 'warn' : 'bad',
      detail: parts.join('；'),
    };
  }

  function recordLine(record) {
    const summary = attemptSummary(record);
    const batchCount = record.batchId
      ? loadStore().records.filter((item) => item.batchId === record.batchId).length
      : 0;
    return h(
      'div',
      { class: 'jp-record' },
      h(
        'div',
        { class: 'jp-record-top' },
        h('span', { class: `jp-badge ${summary.cls}` }, summary.label),
        h('span', {}, formatDate(record.capturedAt)),
        h('span', { class: 'jp-muted' }, record.source === 'exam' ? '整套抓取' : '单题保存'),
        record.access?.explanation === 'locked' ? h('span', { class: 'jp-badge warn' }, '解析未解锁') : null,
        record.access?.transcript === 'locked' ? h('span', { class: 'jp-badge warn' }, '原文未解锁') : null,
      ),
      h('div', { class: 'jp-record-detail' }, summary.detail),
      h(
        'div',
        { class: 'jp-record-actions' },
        record.batchId
          ? h(
              'button',
              {
                class: 'jp-link',
                'data-action': 'select-batch',
                'data-batch': record.batchId,
              },
              `同批抓取 ${batchCount} 条`,
            )
          : null,
        record.batchId
          ? h(
              'button',
              { class: 'jp-link', 'data-action': 'export-batch', 'data-batch': record.batchId },
              '导出本批',
            )
          : null,
        record.batchId
          ? h(
              'button',
              {
                class: 'jp-link jp-danger',
                'data-action': 'delete-batch',
                'data-batch': record.batchId,
              },
              '删除本批',
            )
          : null,
        h(
          'button',
          { class: 'jp-link', 'data-action': 'export-record', 'data-record': record.id },
          '导出本条',
        ),
        h(
          'button',
          { class: 'jp-link jp-danger', 'data-action': 'delete-record', 'data-record': record.id },
          '删除本条',
        ),
      ),
    );
  }

  function renderQuestion(record, records, paperLabel) {
    const key = questionKey(record);
    const groups = record.question.groups || [];
    const summary = attemptSummary(record);
    const expanded = state.expandedQuestions.has(key);
    const latest = records[0];
    return h(
      'div',
      { class: 'jp-question' },
      h(
        'label',
        { class: 'jp-check' },
        h('input', {
          type: 'checkbox',
          'data-qkey': key,
          checked: state.selected.has(key),
          'aria-label': '选择这道题',
        }),
      ),
      h(
        'div',
        { class: 'jp-q-main' },
        h(
          'div',
          { class: 'jp-q-title' },
          h('span', {}, `第${record.question.index}题 · ${record.question.labels.chapter}`),
          h('span', { class: `jp-badge ${summary.cls}` }, summary.label),
          groups.length > 1 ? h('span', { class: 'jp-badge' }, `${groups.length} 小题`) : null,
        ),
        h(
          'div',
          { class: 'jp-q-sub' },
          state.view === 'type' ? h('span', {}, paperLabel) : null,
          h('span', {}, summary.detail),
          latest.attempt.siteStats
            ? h(
                'span',
                { class: 'jp-muted' },
                `网站计数 ${latest.attempt.siteStats.correct} 对 / ${latest.attempt.siteStats.wrong} 错`,
              )
            : null,
          h('span', { class: 'jp-muted' }, formatDate(latest.capturedAt)),
        ),
        h(
          'div',
          { class: 'jp-q-actions' },
          h(
            'button',
            { class: 'jp-link', 'data-action': 'toggle-records', 'data-qkey': key },
            expanded ? '收起记录' : `${records.length} 条记录`,
          ),
          h(
            'button',
            { class: 'jp-link', 'data-action': 'export-question', 'data-qkey': key },
            '导出本题',
          ),
          h(
            'button',
            { class: 'jp-link jp-danger', 'data-action': 'delete-question', 'data-qkey': key },
            '删除本题',
          ),
        ),
        expanded ? h('div', {}, records.map((item) => recordLine(item))) : null,
      ),
    );
  }

  function groupHeader({ scope, value, title, questionKeys, records }) {
    const groupKey = `${scope}:${value}`;
    const expanded = state.expandedGroups.has(groupKey);
    const allSelected = questionKeys.length > 0 && questionKeys.every((key) => state.selected.has(key));
    return h(
      'div',
      { class: 'jp-group-head' },
      h(
        'button',
        { class: 'jp-group-toggle', 'data-action': 'toggle-group', 'data-group': groupKey },
        h('span', { class: `jp-chevron ${expanded ? 'open' : ''}` }, icon('chevron')),
        h('span', { class: 'jp-group-title' }, title),
        h('span', { class: 'jp-muted' }, `${questionKeys.length} 道题 · ${records.length} 条记录`),
      ),
      h(
        'div',
        { class: 'jp-group-actions' },
        h(
          'button',
          { class: 'jp-link', 'data-action': 'select-group', 'data-group': groupKey },
          allSelected ? '取消全选' : '全选',
        ),
        h(
          'button',
          { class: 'jp-link', 'data-action': 'export-group', 'data-group': groupKey },
          scope === 'paper' ? '导出本卷' : '导出本类',
        ),
        h(
          'button',
          { class: 'jp-link jp-danger', 'data-action': 'delete-group', 'data-group': groupKey },
          scope === 'paper' ? '删除本卷' : '删除本类',
        ),
      ),
    );
  }

  function renderCollection() {
    let store;
    try {
      store = loadStore();
    } catch (error) {
      return h(
        'div',
        { class: 'jp-overlay' },
        h(
          'div',
          { class: 'jp-modal', role: 'dialog', 'aria-modal': 'true' },
          h('div', { class: 'jp-head' }, h('h2', {}, '收藏集'), h('button', { class: 'jp-icon', 'data-action': 'close' }, iconNode('close'))),
          h('div', { class: 'jp-body' }, h('p', { class: 'jp-error' }, error.message)),
        ),
      );
    }

    const groups = groupQuestions(store.records);
    const questionKeys = [...groups.keys()].sort((a, b) => compareQuestionKeys(a, b, groups));
    const paperMap = new Map();
    const typeMap = new Map();
    for (const key of questionKeys) {
      const record = groups.get(key)[0];
      const pKey = paperKey(record);
      const tKey = typeKey(record);
      if (!paperMap.has(pKey)) paperMap.set(pKey, []);
      paperMap.get(pKey).push(key);
      if (!typeMap.has(tKey)) typeMap.set(tKey, []);
      typeMap.get(tKey).push(key);
    }

    const paperEntries = [...paperMap.entries()].sort((a, b) => b[0].localeCompare(a[0]));
    const typeEntries = [...typeMap.entries()].sort((a, b) => {
      const left = groups.get(a[1][0])[0];
      const right = groups.get(b[1][0])[0];
      const category = (CATEGORY_ORDER[left.question.category] ?? 99) - (CATEGORY_ORDER[right.question.category] ?? 99);
      if (category) return category;
      return (CHAPTER_ORDER[left.question.chapter] ?? 99) - (CHAPTER_ORDER[right.question.chapter] ?? 99);
    });

    const entries = state.view === 'paper'
      ? paperEntries.map(([key, keys]) => ({
          scope: 'paper',
          value: key,
          title: paperTitle(groups.get(keys[0])[0]),
          keys,
        }))
      : typeEntries.map(([key, keys]) => {
          const record = groups.get(keys[0])[0];
          return {
            scope: 'type',
            value: key,
            title: `${record.question.labels.category} · ${record.question.labels.chapter}`,
            keys,
          };
        });

    const currentPaper = currentPaperKey();
    let expandedKey = null;
    if (state.view === 'paper') {
      expandedKey = currentPaper && paperMap.has(currentPaper)
        ? `paper:${currentPaper}`
        : paperEntries.length ? `paper:${paperEntries[0][0]}` : null;
    } else {
      const routeType = state.route?.category ? `${state.route.category}/${state.route.chapter}` : null;
      expandedKey = routeType && typeMap.has(routeType)
        ? `type:${routeType}`
        : typeEntries.length ? `type:${typeEntries[0][0]}` : null;
    }
    if (expandedKey && !state.expandedGroups.size) state.expandedGroups.add(expandedKey);

    const totalRecords = store.records.length;
    const selectedCount = state.selected.size;

    const overlay = h(
      'div',
      { class: 'jp-overlay' },
      h(
        'div',
        { class: 'jp-modal jp-collection', role: 'dialog', 'aria-modal': 'true', 'aria-label': '收藏集' },
        h(
          'div',
          { class: 'jp-head' },
          h(
            'div',
            {},
            h('h2', {}, '收藏集'),
            h('p', { class: 'jp-muted' }, `${questionKeys.length} 道题 · ${totalRecords} 条记录 · ${formatBytes(storeSize())}`),
          ),
          h('button', { class: 'jp-icon', 'data-action': 'close', 'aria-label': '关闭' }, iconNode('close')),
        ),
        h(
          'div',
          { class: 'jp-toolbar' },
          h(
            'div',
            { class: 'jp-tabs' },
            h(
              'button',
              { class: state.view === 'paper' ? 'active' : '', 'data-action': 'view-paper' },
              '按试卷',
            ),
            h(
              'button',
              { class: state.view === 'type' ? 'active' : '', 'data-action': 'view-type' },
              '按题型',
            ),
          ),
          h(
            'div',
            { class: 'jp-toolbar-actions' },
            h('span', { class: 'jp-muted' }, `已选 ${selectedCount} 道题`),
            h(
              'button',
              {
                class: 'jp-btn',
                'data-action': 'export-selected',
                disabled: selectedCount === 0,
              },
              '导出所选',
            ),
            h(
              'button',
              {
                class: 'jp-btn jp-danger',
                'data-action': 'delete-selected',
                disabled: selectedCount === 0,
              },
              '删除所选',
            ),
            h('button', { class: 'jp-btn', 'data-action': 'export-all' }, '导出全部'),
          ),
        ),
        h(
          'div',
          { class: 'jp-body' },
          entries.length
            ? entries.map((entry) => {
                const groupKey = `${entry.scope}:${entry.value}`;
                const expanded = state.expandedGroups.has(groupKey);
                const groupRecords = entry.keys.flatMap((key) => groups.get(key));
                return h(
                  'div',
                  { class: 'jp-group', 'data-group': groupKey },
                  groupHeader({
                    scope: entry.scope,
                    value: entry.value,
                    title: entry.title,
                    questionKeys: entry.keys,
                    records: groupRecords,
                  }),
                  expanded
                    ? h(
                        'div',
                        { class: 'jp-group-body' },
                        entry.keys.map((key) => {
                          const records = groups.get(key);
                          const paper = paperTitle(records[0]);
                          return renderQuestion(records[0], records, paper);
                        }),
                      )
                    : null,
                );
              })
            : h(
                'div',
                { class: 'jp-empty' },
                h('p', {}, '收藏集是空的。'),
                h('p', { class: 'jp-muted' }, '打开一道题目，点击右侧的加号保存；或打开一套试卷，抓取整套。'),
              ),
        ),
        h(
          'div',
          { class: 'jp-foot' },
          h('span', { class: 'jp-muted' }, '导出文件为 JSON，可交给 JaLaQuery 查询和判分。'),
        ),
      ),
    );
    return overlay;
  }

  // ---------- 存储界面 ----------

  function storageEntries(kind) {
    const storage = kind === 'session' ? sessionStorage : localStorage;
    return Object.keys(storage)
      .sort()
      .map((key) => {
        const value = storage.getItem(key) ?? '';
        return {
          key,
          value,
          size: textSize(key + value),
          ours: key.startsWith('jalaplus:'),
        };
      });
  }

  function storageTotal(kind) {
    return storageEntries(kind).reduce((sum, entry) => sum + entry.size, 0);
  }

  function exportStorage() {
    const local = {};
    const session = {};
    for (const key of Object.keys(localStorage)) local[key] = localStorage.getItem(key) ?? '';
    for (const key of Object.keys(sessionStorage)) session[key] = sessionStorage.getItem(key) ?? '';
    const payload = {
      format: 'jalaplus.storage',
      version: 1,
      exportedAt: new Date().toISOString(),
      localStorage: local,
      sessionStorage: session,
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `jalaplus-storage-${new Date().toISOString().slice(0, 10)}.json`;
    link.style.display = 'none';
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 2000);
    toast('已导出存储内容', 'success');
  }

  function renderStorage() {
    const entries = storageEntries(state.storageTab);
    const total = storageTotal(state.storageTab);
    const localTotal = storageTotal('local');
    const sessionTotal = storageTotal('session');
    let recordCount = '读取失败';
    try {
      recordCount = String(loadStore().records.length);
    } catch {
      // 收藏集损坏时仍允许管理站点存储
    }
    return h(
      'div',
      { class: 'jp-overlay' },
      h(
        'div',
        { class: 'jp-modal jp-storage', role: 'dialog', 'aria-modal': 'true', 'aria-label': '存储' },
        h(
          'div',
          { class: 'jp-head' },
          h(
            'div',
            {},
            h('h2', {}, '存储'),
            h('p', { class: 'jp-muted' }, `localStorage ${formatBytes(localTotal)} · sessionStorage ${formatBytes(sessionTotal)}`),
          ),
          h('button', { class: 'jp-icon', 'data-action': 'close', 'aria-label': '关闭' }, iconNode('close')),
        ),
        h(
          'div',
          { class: 'jp-toolbar' },
          h(
            'div',
            { class: 'jp-tabs' },
            h(
              'button',
              { class: state.storageTab === 'local' ? 'active' : '', 'data-action': 'storage-local' },
              'localStorage',
            ),
            h(
              'button',
              { class: state.storageTab === 'session' ? 'active' : '', 'data-action': 'storage-session' },
              'sessionStorage',
            ),
          ),
          h(
            'div',
            { class: 'jp-toolbar-actions' },
            h('button', { class: 'jp-btn', 'data-action': 'storage-refresh' }, iconNode('refresh'), '刷新'),
            h('button', { class: 'jp-btn', 'data-action': 'storage-export' }, iconNode('download'), '导出'),
            h('button', { class: 'jp-btn', 'data-action': 'storage-clear-stats' }, '清除作答统计'),
            h('button', { class: 'jp-btn jp-danger', 'data-action': 'storage-clear-site' }, '清除站点数据'),
          ),
        ),
        h(
          'div',
          { class: 'jp-body' },
          h(
            'p',
            { class: 'jp-muted' },
            `当前 ${state.storageTab === 'local' ? 'localStorage' : 'sessionStorage'} 共 ${entries.length} 个键，${formatBytes(total)}。只显示浏览器存储，不包含 Cookie。`,
          ),
          h(
            'p',
            { class: 'jp-muted' },
            `JaLaPlus 收藏集：${recordCount} 条记录，${formatBytes(storeSize())}。`,
          ),
          entries.length
            ? entries.map((entry) => {
                const expanded = state.expandedKeys.has(entry.key);
                return h(
                  'div',
                  { class: 'jp-key' },
                  h(
                    'div',
                    { class: 'jp-key-head' },
                    h('code', {}, entry.key),
                    entry.ours ? h('span', { class: 'jp-badge' }, 'JaLaPlus') : null,
                    h('span', { class: 'jp-muted' }, formatBytes(entry.size)),
                    h(
                      'button',
                      {
                        class: 'jp-link',
                        'data-action': 'toggle-key',
                        'data-key': entry.key,
                      },
                      expanded ? '收起' : '查看',
                    ),
                    h(
                      'button',
                      {
                        class: 'jp-link jp-danger',
                        'data-action': 'delete-key',
                        'data-key': entry.key,
                      },
                      '删除',
                    ),
                  ),
                  expanded
                    ? h('pre', { class: 'jp-pre' }, entry.value || '（空）')
                    : h('div', { class: 'jp-key-preview' }, entry.value.slice(0, 180) || '（空）'),
                );
              })
            : h('p', { class: 'jp-empty' }, '没有数据。'),
        ),
        h(
          'div',
          { class: 'jp-foot' },
          h('span', { class: 'jp-muted' }, '清除站点数据不会删除 JaLaPlus 的收藏集。'),
        ),
      ),
    );
  }

  // ---------- 弹窗与事件 ----------

  function renderModalRoot({ preserveScroll = true } = {}) {
    if (!shell) return;
    const body = shell.modalRoot.querySelector('.jp-body');
    const scroll = preserveScroll && body ? body.scrollTop : 0;
    shell.modalRoot.replaceChildren();
    if (state.modalKind === 'collection') shell.modalRoot.append(renderCollection());
    else if (state.modalKind === 'storage') shell.modalRoot.append(renderStorage());
    else if (state.modalKind === 'progress' && state.progress) shell.modalRoot.append(renderProgress());
    const nextBody = shell.modalRoot.querySelector('.jp-body');
    if (nextBody && preserveScroll) nextBody.scrollTop = scroll;
  }

  function closeModal() {
    if (state.activeConfirm) {
      state.activeConfirm(false);
      state.activeConfirm = null;
    }
    state.modalKind = null;
    state.progress = null;
    if (shell) shell.modalRoot.replaceChildren();
  }

  function openCollection() {
    try {
      loadStore();
    } catch (error) {
      toast(error.message, 'error');
      return;
    }
    state.modalKind = 'collection';
    state.selected.clear();
    state.expandedGroups.clear();
    state.expandedQuestions.clear();
    state.view = 'paper';
    const store = loadStore();
    const paper = currentPaperKey();
    if (paper && store.records.some((record) => paperKey(record) === paper)) {
      state.expandedGroups.add(`paper:${paper}`);
    }
    renderModalRoot({ preserveScroll: false });
  }

  function renderProgress() {
    const progress = state.progress;
    const percent = progress.total ? Math.round((progress.done / progress.total) * 100) : 0;
    return h(
      'div',
      { class: 'jp-overlay' },
      h(
        'div',
        { class: 'jp-modal jp-progress', role: 'dialog', 'aria-modal': 'true', 'aria-label': '抓取进度' },
        h('div', { class: 'jp-head' }, h('h2', {}, progress.title)),
        h(
          'div',
          { class: 'jp-body' },
          h('p', {}, `正在读取 ${progress.done} / ${progress.total}`),
          h(
            'div',
            { class: 'jp-progress-bar' },
            h('div', { class: 'jp-progress-fill', style: `width:${percent}%` }),
          ),
          h(
            'p',
            { class: 'jp-muted' },
            `成功 ${progress.ok} 条，失败 ${progress.failed} 条${progress.cancelled ? '，正在停止…' : ''}`,
          ),
          h(
            'button',
            { class: 'jp-btn', 'data-action': 'cancel-batch', disabled: progress.cancelled },
            '取消',
          ),
        ),
      ),
    );
  }

  function askConfirm(message, confirmLabel = '删除', danger = true) {
    if (!shell) return Promise.resolve(false);
    return new Promise((resolve) => {
      const finish = (value) => {
        state.activeConfirm = null;
        overlay.remove();
        resolve(value);
      };
      const overlay = h(
        'div',
        { class: 'jp-overlay jp-confirm' },
        h(
          'div',
          { class: 'jp-modal jp-confirm-box', role: 'alertdialog', 'aria-modal': 'true' },
          h('div', { class: 'jp-head' }, h('h2', {}, confirmLabel === '开始抓取' ? '抓取整套试卷' : '确认操作')),
          h('div', { class: 'jp-body' }, h('p', {}, message)),
          h(
            'div',
            { class: 'jp-confirm-actions' },
            h('button', { class: 'jp-btn', 'data-role': 'cancel' }, '取消'),
            h('button', { class: `jp-btn ${danger ? 'jp-danger-solid' : 'jp-primary'}`, 'data-role': 'confirm' }, confirmLabel),
          ),
        ),
      );
      overlay.addEventListener('click', (event) => {
        if (event.target === overlay) finish(false);
      });
      overlay.querySelector('[data-role="cancel"]').addEventListener('click', () => finish(false));
      overlay.querySelector('[data-role="confirm"]').addEventListener('click', () => finish(true));
      state.activeConfirm = finish;
      shell.modalRoot.append(overlay);
      overlay.querySelector('[data-role="confirm"]').focus();
    });
  }

  function recordsForGroup(scope, value) {
    const store = loadStore();
    return store.records.filter((record) =>
      scope === 'paper' ? paperKey(record) === value : typeKey(record) === value,
    );
  }

  function questionKeysForGroup(scope, value) {
    return [...new Set(recordsForGroup(scope, value).map(questionKey))];
  }

  function selectedRecords() {
    const store = loadStore();
    return store.records.filter((record) => state.selected.has(questionKey(record)));
  }

  function mutateStore(mutator) {
    const store = loadStore();
    mutator(store);
    saveStore(store);
  }

  async function deleteRecords(records, message) {
    if (!records.length) return;
    const confirmed = await askConfirm(`${message}（共 ${records.length} 条记录）`, '删除');
    if (!confirmed) return;
    const ids = new Set(records.map((record) => record.id));
    mutateStore((store) => {
      store.records = store.records.filter((record) => !ids.has(record.id));
    });
    const remainingKeys = new Set(loadStore().records.map(questionKey));
    for (const key of [...state.selected]) {
      if (!remainingKeys.has(key)) state.selected.delete(key);
    }
    toast(`已删除 ${ids.size} 条记录`, 'success');
    renderModalRoot();
  }

  function downloadSelected(kind, description, slug, records) {
    if (!records.length) {
      toast('没有可导出的记录', 'info');
      return;
    }
    exportFile(records, kind, description, slug);
  }

  async function handleCollectionAction(action, element) {
    switch (action) {
      case 'close':
        closeModal();
        break;
      case 'view-paper':
      case 'view-type': {
        state.view = action === 'view-paper' ? 'paper' : 'type';
        state.expandedGroups.clear();
        state.expandedQuestions.clear();
        const store = loadStore();
        if (state.view === 'paper') {
          const paper = currentPaperKey();
          const fallback = [...store.records].sort((a, b) => paperKey(b).localeCompare(paperKey(a)))[0];
          const key = paper || (fallback ? paperKey(fallback) : null);
          if (key) state.expandedGroups.add(`paper:${key}`);
        } else if (state.route?.category) {
          state.expandedGroups.add(`type:${state.route.category}/${state.route.chapter}`);
        }
        renderModalRoot({ preserveScroll: false });
        break;
      }
      case 'toggle-group': {
        const key = element.dataset.group;
        if (state.expandedGroups.has(key)) state.expandedGroups.delete(key);
        else state.expandedGroups.add(key);
        renderModalRoot();
        break;
      }
      case 'toggle-records': {
        const key = element.dataset.qkey;
        if (state.expandedQuestions.has(key)) state.expandedQuestions.delete(key);
        else state.expandedQuestions.add(key);
        renderModalRoot();
        break;
      }
      case 'select-group': {
        const scope = element.dataset.group.split(':')[0];
        const value = element.dataset.group.slice(scope.length + 1);
        const keys = questionKeysForGroup(scope, value);
        const all = keys.length > 0 && keys.every((key) => state.selected.has(key));
        for (const key of keys) {
          if (all) state.selected.delete(key);
          else state.selected.add(key);
        }
        renderModalRoot();
        break;
      }
      case 'select-batch': {
        const batchId = element.dataset.batch;
        const store = loadStore();
        const keys = [...new Set(store.records.filter((record) => record.batchId === batchId).map(questionKey))];
        const all = keys.length > 0 && keys.every((key) => state.selected.has(key));
        for (const key of keys) {
          if (all) state.selected.delete(key);
          else state.selected.add(key);
        }
        renderModalRoot();
        break;
      }
      case 'export-selected':
        downloadSelected(
          'selection',
          `已选 ${state.selected.size} 道题`,
          'selection',
          selectedRecords(),
        );
        break;
      case 'export-all': {
        const store = loadStore();
        downloadSelected('all', '全部收藏', 'all', store.records);
        break;
      }
      case 'export-question': {
        const key = element.dataset.qkey;
        const store = loadStore();
        const records = store.records.filter((record) => questionKey(record) === key);
        const sample = records[0];
        downloadSelected(
          'selection',
          sample ? `第${sample.question.index}题 · ${sample.question.labels.chapter}` : '单题',
          key.split('/').slice(0, 2).join('-') + '-' + key.split('/').slice(2).join('-'),
          records,
        );
        break;
      }
      case 'export-group': {
        const scope = element.dataset.group.split(':')[0];
        const value = element.dataset.group.slice(scope.length + 1);
        const records = recordsForGroup(scope, value);
        downloadSelected(
          scope === 'paper' ? 'paper' : 'type',
          scope === 'paper' ? `试卷 ${value}` : `题型 ${value}`,
          value.replace(/[^a-z0-9-]+/gi, '-'),
          records,
        );
        break;
      }
      case 'export-batch': {
        const batchId = element.dataset.batch;
        const records = loadStore().records.filter((record) => record.batchId === batchId);
        downloadSelected('batch', `本次抓取的 ${records.length} 条记录`, 'batch', records);
        break;
      }
      case 'export-record': {
        const id = element.dataset.record;
        const store = loadStore();
        const record = store.records.find((item) => item.id === id);
        if (record) {
          const key = `${paperKey(record).replace('/', '-')}-${record.question.category}-${record.question.chapter}-${record.question.index}`;
          downloadSelected('record', `第${record.question.index}题的一条记录`, key, [record]);
        }
        break;
      }
      case 'delete-selected':
        await deleteRecords(selectedRecords(), '删除所选题目');
        break;
      case 'delete-question': {
        const key = element.dataset.qkey;
        const records = loadStore().records.filter((record) => questionKey(record) === key);
        await deleteRecords(records, '删除这道题');
        break;
      }
      case 'delete-group': {
        const scope = element.dataset.group.split(':')[0];
        const value = element.dataset.group.slice(scope.length + 1);
        await deleteRecords(recordsForGroup(scope, value), scope === 'paper' ? '删除这套试卷' : '删除这个题型');
        break;
      }
      case 'delete-batch': {
        const batchId = element.dataset.batch;
        const records = loadStore().records.filter((record) => record.batchId === batchId);
        await deleteRecords(records, '删除这次抓取的记录');
        break;
      }
      case 'delete-record': {
        const id = element.dataset.record;
        const records = loadStore().records.filter((record) => record.id === id);
        await deleteRecords(records, '删除这条记录');
        break;
      }
      default:
        break;
    }
  }

  async function handleStorageAction(action, element) {
    switch (action) {
      case 'close':
        closeModal();
        break;
      case 'storage-local':
      case 'storage-session':
        state.storageTab = action === 'storage-local' ? 'local' : 'session';
        state.expandedKeys.clear();
        renderModalRoot({ preserveScroll: false });
        break;
      case 'storage-refresh':
        renderModalRoot({ preserveScroll: false });
        break;
      case 'storage-export':
        exportStorage();
        break;
      case 'toggle-key': {
        const key = element.dataset.key;
        if (state.expandedKeys.has(key)) state.expandedKeys.delete(key);
        else state.expandedKeys.add(key);
        renderModalRoot();
        break;
      }
      case 'delete-key': {
        const key = element.dataset.key;
        const storage = state.storageTab === 'session' ? sessionStorage : localStorage;
        const confirmed = await askConfirm(`删除键“${key}”？`, '删除');
        if (!confirmed) return;
        storage.removeItem(key);
        state.expandedKeys.delete(key);
        toast('已删除该键', 'success');
        renderModalRoot();
        break;
      }
      case 'storage-clear-stats': {
        const storage = state.storageTab === 'session' ? sessionStorage : localStorage;
        const keys = Object.keys(storage).filter((key) => key.startsWith('jlpt:stats:'));
        const confirmed = await askConfirm(`清除 ${keys.length} 个作答统计键？`, '清除');
        if (!confirmed) return;
        for (const key of keys) storage.removeItem(key);
        toast(`已清除 ${keys.length} 个统计键`, 'success');
        renderModalRoot();
        break;
      }
      case 'storage-clear-site': {
        const storage = state.storageTab === 'session' ? sessionStorage : localStorage;
        const keys = Object.keys(storage).filter((key) => !key.startsWith('jalaplus:'));
        const confirmed = await askConfirm(
          `清除 ${keys.length} 个站点数据键？JaLaPlus 收藏集不受影响。`,
          '清除',
        );
        if (!confirmed) return;
        for (const key of keys) storage.removeItem(key);
        toast(`已清除 ${keys.length} 个键`, 'success');
        renderModalRoot();
        break;
      }
      default:
        break;
    }
  }

  async function handleModalClick(event) {
    const target = event.target.closest('[data-action]');
    if (!target) return;
    const action = target.dataset.action;
    try {
      if (action === 'cancel-batch') {
        state.batchCancel = true;
        if (state.progress) state.progress.cancelled = true;
        renderModalRoot({ preserveScroll: false });
        return;
      }
      if (action === 'close') {
        closeModal();
        return;
      }
      if (action.startsWith('storage-') || action === 'toggle-key' || action === 'delete-key') {
        await handleStorageAction(action, target);
        return;
      }
      await handleCollectionAction(action, target);
    } catch (error) {
      toast(`操作失败：${error.message}`, 'error');
    }
  }

  function handleModalChange(event) {
    const input = event.target;
    if (!input || input.tagName !== 'INPUT' || input.type !== 'checkbox') return;
    const key = input.dataset.qkey;
    if (!key) return;
    if (input.checked) state.selected.add(key);
    else state.selected.delete(key);
    const count = shell.modalRoot.querySelector('.jp-toolbar-actions .jp-muted');
    if (count) count.textContent = `已选 ${state.selected.size} 道题`;
    for (const button of shell.modalRoot.querySelectorAll('[data-action="export-selected"], [data-action="delete-selected"]')) {
      button.disabled = state.selected.size === 0;
    }
  }

  function refreshRoute() {
    if (!shell) return;
    const route = parseRoute();
    const changed = JSON.stringify(route) !== JSON.stringify(state.route);
    state.route = route;
    shell.host.style.display = route ? 'block' : 'none';
    if (!route) {
      if (state.modalKind) closeModal();
      return;
    }
    if (changed && state.modalKind && state.modalKind !== 'progress') {
      closeModal();
    }
    const capture = shell.fabWrap.querySelector('[data-action="capture"]');
    if (capture) {
      const label = route.category && route.index
        ? '保存当前题目'
        : route.level && route.exam && !route.category
          ? '抓取整套试卷'
          : '请先打开试卷或题目';
      capture.title = label;
      capture.setAttribute('aria-label', label);
    }
    if (!document.body.contains(shell.host)) document.body.append(shell.host);
  }

  // ---------- 初始化 ----------

  function ensureShell() {
    if (shell) return shell;
    const host = document.createElement('div');
    host.id = HOST_ID;
    host.style.cssText = 'position:fixed;inset:0;pointer-events:none;z-index:2147483000;';

    const shadow = host.attachShadow({ mode: 'open' });
    const style = document.createElement('style');
    style.textContent = STYLES;

    const fabWrap = h(
      'div',
      { class: 'jp-fab-wrap' },
      h(
        'button',
        { class: 'jp-fab jp-fab-primary', 'data-action': 'capture', title: '保存当前题目', 'aria-label': '保存当前题目' },
        iconNode('plus'),
      ),
      h(
        'button',
        { class: 'jp-fab', 'data-action': 'collection', title: '查看收藏集', 'aria-label': '查看收藏集' },
        iconNode('collection'),
      ),
      h(
        'button',
        { class: 'jp-fab', 'data-action': 'storage', title: '操作存储', 'aria-label': '操作存储' },
        iconNode('storage'),
      ),
    );
    const toastWrap = h('div', { class: 'jp-toast-wrap', 'aria-live': 'polite' });
    const modalRoot = h('div', { class: 'jp-modal-root' });
    shadow.append(style, fabWrap, toastWrap, modalRoot);

    const append = () => {
      if (document.body) document.body.append(host);
      else document.addEventListener('DOMContentLoaded', () => document.body.append(host), { once: true });
    };
    append();

    fabWrap.addEventListener('click', (event) => {
      const button = event.target.closest('[data-action]');
      if (!button) return;
      if (button.dataset.action === 'capture') {
        const route = parseRoute();
        if (route && route.category && route.index) void captureQuestion();
        else if (route && route.level && route.exam && !route.category) void capturePaper();
        else toast('请先打开一套试卷或一道题目', 'info');
      } else if (button.dataset.action === 'collection') {
        openCollection();
      } else if (button.dataset.action === 'storage') {
        try {
          loadStore();
        } catch (error) {
          toast(error.message, 'error');
          return;
        }
        state.modalKind = 'storage';
        state.storageTab = 'local';
        state.expandedKeys.clear();
        renderModalRoot({ preserveScroll: false });
      }
    });

    modalRoot.addEventListener('click', (event) => {
      const overlay = event.target;
      if (overlay.classList && overlay.classList.contains('jp-overlay') && !overlay.classList.contains('jp-confirm')) {
        if (state.modalKind === 'progress') return;
        closeModal();
        return;
      }
      void handleModalClick(event);
    });
    modalRoot.addEventListener('change', handleModalChange);

    document.addEventListener('keydown', (event) => {
      if (event.key !== 'Escape') return;
      if (state.activeConfirm) {
        state.activeConfirm(false);
        state.activeConfirm = null;
        return;
      }
      if (state.modalKind === 'progress' && state.batchRunning) {
        state.batchCancel = true;
        if (state.progress) state.progress.cancelled = true;
        renderModalRoot({ preserveScroll: false });
        return;
      }
      if (state.modalKind) closeModal();
    });

    window.addEventListener('popstate', refreshRoute);
    window.addEventListener('storage', (event) => {
      if (event.key === STORAGE_KEY && state.modalKind === 'collection') renderModalRoot();
    });

    const observer = new MutationObserver(() => {
      if (!document.body.contains(host)) document.body.append(host);
    });
    observer.observe(document.body, { childList: true });

    shell = { host, shadow, fabWrap, toastWrap, modalRoot };
    return shell;
  }

  const STYLES = `
    * { box-sizing: border-box; }
    button { font: inherit; cursor: pointer; }
    .jp-fab-wrap {
      position: fixed; right: 20px; bottom: 128px;
      display: flex; flex-direction: column; gap: 10px;
      z-index: 2147483001; pointer-events: none;
    }
    .jp-fab {
      pointer-events: auto; width: 52px; height: 52px; padding: 0;
      display: inline-flex; align-items: center; justify-content: center;
      border-radius: 50%; border: 1px solid #e2e8f0; background: #fff;
      color: #e26a8d; box-shadow: 0 4px 14px rgba(15, 23, 42, .12);
      transition: background .15s, border-color .15s, transform .15s;
    }
    .jp-fab:hover { background: #fdf2f6; border-color: #e26a8d; transform: translateY(-1px); }
    .jp-fab-primary { background: #e26a8d; border-color: #e26a8d; color: #fff; }
    .jp-fab-primary:hover { background: #d85b80; border-color: #d85b80; }
    .jp-icon-svg { display: inline-flex; width: 24px; height: 24px; }
    .jp-icon-svg svg { width: 100%; height: 100%; }
    .jp-overlay {
      position: fixed; inset: 0; background: rgba(15, 23, 42, .34);
      display: flex; align-items: center; justify-content: center;
      padding: 16px; pointer-events: auto; z-index: 2147483002;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "Microsoft YaHei", sans-serif;
      color: #1e293b; font-size: 14px; line-height: 1.5;
    }
    .jp-modal {
      width: min(920px, 100%); max-height: min(86vh, 900px);
      display: flex; flex-direction: column; overflow: hidden;
      background: #fff; border: 1px solid #e2e8f0; border-radius: 16px;
      box-shadow: 0 18px 48px rgba(15, 23, 42, .2);
    }
    .jp-collection { width: min(980px, 100%); }
    .jp-storage { width: min(880px, 100%); }
    .jp-progress { width: min(440px, 100%); }
    .jp-confirm { z-index: 2147483003; }
    .jp-confirm-box { width: min(420px, 100%); }
    .jp-head {
      display: flex; align-items: flex-start; justify-content: space-between;
      gap: 12px; padding: 18px 20px 14px; border-bottom: 1px solid #e2e8f0;
    }
    .jp-head h2 { margin: 0; font-size: 18px; font-weight: 650; letter-spacing: .01em; }
    .jp-head p { margin: 4px 0 0; }
    .jp-muted { color: #64748b; font-size: 12px; }
    .jp-error { color: #b91c1c; }
    .jp-toolbar {
      display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between;
      gap: 10px 14px; padding: 12px 20px; border-bottom: 1px solid #e2e8f0; background: #fff;
    }
    .jp-tabs { display: inline-flex; gap: 4px; padding: 3px; background: #f1f5f9; border-radius: 9px; }
    .jp-tabs button {
      border: 1px solid transparent; background: transparent; color: #475569;
      padding: 5px 12px; border-radius: 7px; font-size: 13px;
    }
    .jp-tabs button.active { background: #fff; color: #e26a8d; border-color: #e2e8f0; font-weight: 600; }
    .jp-toolbar-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
    .jp-btn {
      display: inline-flex; align-items: center; gap: 5px;
      border: 1px solid #e2e8f0; background: #fff; color: #334155;
      padding: 6px 11px; border-radius: 8px; font-size: 13px;
      transition: background .15s, border-color .15s;
    }
    .jp-btn:hover { background: #f8fafc; border-color: #cbd5e1; }
    .jp-btn:disabled { opacity: .45; cursor: not-allowed; }
    .jp-btn .jp-icon-svg { width: 16px; height: 16px; }
    .jp-danger { color: #b91c1c; border-color: #fecaca; }
    .jp-danger:hover { background: #fef2f2; border-color: #fca5a5; }
    .jp-danger-solid { background: #dc2626; border-color: #dc2626; color: #fff; }
    .jp-danger-solid:hover { background: #b91c1c; }
    .jp-primary { background: #e26a8d; border-color: #e26a8d; color: #fff; }
    .jp-primary:hover { background: #d85b80; }
    .jp-icon {
      width: 32px; height: 32px; padding: 0; display: inline-flex;
      align-items: center; justify-content: center; border: 0; background: transparent;
      color: #64748b; border-radius: 8px;
    }
    .jp-icon:hover { background: #f1f5f9; color: #1e293b; }
    .jp-body { overflow: auto; padding: 8px 20px 16px; }
    .jp-foot { border-top: 1px solid #e2e8f0; padding: 10px 20px; background: #f8fafc; }
    .jp-group { border: 1px solid #e2e8f0; border-radius: 12px; margin: 12px 0; overflow: hidden; }
    .jp-group-head { display: flex; align-items: center; gap: 8px; padding: 9px 12px; background: #f8fafc; }
    .jp-group-toggle {
      display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0;
      border: 0; background: transparent; padding: 0; text-align: left; color: #1e293b;
    }
    .jp-group-title { font-weight: 600; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .jp-chevron { display: inline-flex; width: 16px; height: 16px; color: #64748b; transition: transform .15s; }
    .jp-chevron.open { transform: rotate(90deg); }
    .jp-chevron svg { width: 100%; height: 100%; }
    .jp-group-actions { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }
    .jp-link {
      border: 0; background: transparent; padding: 2px 0; color: #475569;
      font-size: 12px; text-decoration: underline; text-underline-offset: 3px;
    }
    .jp-link:hover { color: #e26a8d; }
    .jp-link.jp-danger { color: #b91c1c; }
    .jp-question { display: flex; gap: 10px; padding: 12px; border-top: 1px solid #e2e8f0; }
    .jp-check { padding-top: 3px; }
    .jp-check input { width: 16px; height: 16px; accent-color: #e26a8d; }
    .jp-q-main { flex: 1; min-width: 0; }
    .jp-q-title { display: flex; flex-wrap: wrap; align-items: center; gap: 7px; font-weight: 600; font-size: 13px; }
    .jp-q-sub { display: flex; flex-wrap: wrap; gap: 4px 10px; margin-top: 4px; color: #475569; font-size: 12px; }
    .jp-q-actions { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 6px; }
    .jp-badge {
      display: inline-block; padding: 0 7px; border-radius: 999px;
      background: #f1f5f9; color: #475569; font-size: 11px; font-weight: 500;
      line-height: 18px; white-space: nowrap;
    }
    .jp-badge.ok { background: #dcfce7; color: #166534; }
    .jp-badge.bad { background: #fee2e2; color: #991b1b; }
    .jp-badge.warn { background: #fef3c7; color: #92400e; }
    .jp-record {
      margin-top: 8px; padding: 8px 10px; background: #f8fafc;
      border: 1px solid #e2e8f0; border-radius: 8px; font-size: 12px; color: #475569;
    }
    .jp-record-top { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 10px; }
    .jp-record-detail { margin-top: 4px; color: #334155; }
    .jp-record-actions { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 6px; }
    .jp-key { border: 1px solid #e2e8f0; border-radius: 10px; padding: 9px 11px; margin: 8px 0; }
    .jp-key-head { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 10px; }
    .jp-key-head code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; color: #334155; }
    .jp-key-head .jp-link { margin-left: auto; }
    .jp-key-head .jp-link + .jp-link { margin-left: 0; }
    .jp-key-preview { margin-top: 5px; color: #64748b; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .jp-pre {
      margin: 8px 0 0; padding: 9px; max-height: 260px; overflow: auto;
      background: #fff; border: 1px solid #e2e8f0; border-radius: 8px;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 12px; line-height: 1.45; white-space: pre-wrap; word-break: break-word;
    }
    .jp-empty { padding: 28px 8px; text-align: center; color: #475569; }
    .jp-progress-bar { height: 8px; background: #f1f5f9; border-radius: 999px; overflow: hidden; margin: 10px 0; }
    .jp-progress-fill { height: 100%; background: #e26a8d; transition: width .15s; }
    .jp-confirm-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 0 20px 18px; }
    .jp-toast-wrap {
      position: fixed; left: 50%; bottom: 24px; transform: translateX(-50%);
      display: flex; flex-direction: column; align-items: center; gap: 8px;
      z-index: 2147483004; pointer-events: none;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "Microsoft YaHei", sans-serif;
    }
    .jp-toast {
      max-width: min(560px, calc(100vw - 32px)); padding: 9px 14px;
      border-radius: 9px; background: #1e293b; color: #fff; font-size: 13px;
      box-shadow: 0 8px 24px rgba(15, 23, 42, .2); animation: jp-in .16s ease-out;
    }
    .jp-toast.success { background: #166534; }
    .jp-toast.error { background: #991b1b; }
    @keyframes jp-in { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: translateY(0); } }
    @media (max-width: 640px) {
      .jp-fab-wrap { right: 12px; bottom: 104px; }
      .jp-fab { width: 46px; height: 46px; }
      .jp-overlay { padding: 8px; }
      .jp-modal { max-height: 92vh; border-radius: 13px; }
      .jp-head, .jp-toolbar, .jp-body, .jp-foot { padding-left: 14px; padding-right: 14px; }
      .jp-group-head { flex-wrap: wrap; }
      .jp-group-actions { margin-left: 24px; }
    }
  `;

  ensureShell();
  refreshRoute();
  setInterval(refreshRoute, 700);
})();
