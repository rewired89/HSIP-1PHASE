/**
 * HSIP Tracker Knowledge Base
 * Sourced from hsip-telemetry-guard/src/known_endpoints.rs
 * Consumer-friendly descriptions added for the Tracker Inspector UI.
 */

export const RISK_LEVEL = {
  critical: { label: 'Critical',  color: '#fc8181', bg: '#63171b' },
  high:     { label: 'High',      color: '#f6ad55', bg: '#7b341e' },
  medium:   { label: 'Medium',    color: '#f6e05e', bg: '#5f370e' },
  low:      { label: 'Low',       color: '#68d391', bg: '#22543d' },
};

export const CATEGORIES = [
  'All',
  'Session Recording',
  'Ad Network',
  'Analytics',
  'Social',
  'Microsoft',
  'Crash Reporting',
];

/** @type {Array<{vendor,domain,plain,description,category,risk,safeToBlock}>} */
export const TRACKERS = [
  // ── Session Recording (highest risk) ─────────────────────────────────
  {
    vendor: 'Hotjar',
    domain: '*.hotjar.com',
    plain: 'Records your screen while you browse',
    description:
      'Hotjar watches everything you do on a website — every mouse move, click, scroll, and ' +
      'keystroke — and sends a full video replay to the website owner. It can capture what you ' +
      'type into forms before you even submit them.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'FullStory',
    domain: '*.fullstory.com',
    plain: 'Records your entire browsing session as a video',
    description:
      'FullStory creates a pixel-perfect recording of your session on any site that uses it. ' +
      'The recording captures mouse movement, clicks, and often text you type. ' +
      'Companies use this to "replay" exactly what you did.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Microsoft Clarity',
    domain: '*.clarity.ms',
    plain: 'Microsoft\'s screen recorder built into many websites',
    description:
      'Microsoft Clarity is a free session-recording tool that millions of websites embed. ' +
      'It captures heatmaps of where people click and full session replays. ' +
      'Because it\'s free, it\'s widely deployed — often without users knowing.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },

  // ── Ad Networks ───────────────────────────────────────────────────────
  {
    vendor: 'Google DoubleClick',
    domain: '*.doubleclick.net',
    plain: 'Google\'s ad network that tracks you across the entire web',
    description:
      'DoubleClick (now Google Ad Manager) is present on millions of websites and builds a ' +
      'profile of every site you visit. This profile is used to show you targeted ads everywhere ' +
      'you go online.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Google AdSense',
    domain: '*.googlesyndication.com',
    plain: 'Tracks which ads you see and whether you click them',
    description:
      'Google AdSense serves ads on websites and tracks impressions, clicks, and your behavior ' +
      'to optimize ad targeting. Every page load with an AdSense ad reports back to Google.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Criteo',
    domain: '*.criteo.com',
    plain: 'Retargeting ads that follow you around the web',
    description:
      'Criteo powers those ads that "follow you" after you look at a product — if you browse ' +
      'shoes on one site, Criteo ensures shoe ads appear on every other site you visit. ' +
      'It does this by tracking your browsing across thousands of partner sites.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Xandr / AppNexus',
    domain: '*.adnxs.com',
    plain: 'Microsoft\'s advertising exchange that tracks your profile',
    description:
      'AppNexus (now Xandr, owned by Microsoft) operates a real-time bidding exchange. ' +
      'Every time a page with their code loads, your browser profile is auctioned to advertisers ' +
      'in milliseconds to decide which ad to show you.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'The Trade Desk',
    domain: '*.adsrvr.org',
    plain: 'Data broker that sells your browsing profile to advertisers',
    description:
      'The Trade Desk aggregates your browsing data from hundreds of sources to build a ' +
      'detailed profile including inferred age, income, interests, and purchase intent. ' +
      'This profile is sold to advertisers.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Amazon Advertising',
    domain: '*.amazon-adsystem.com',
    plain: 'Amazon tracks your browsing to target ads everywhere',
    description:
      'Amazon\'s ad platform tracks you on sites outside Amazon to show you ads based on your ' +
      'shopping history and browsing behavior. Even if you\'re not on Amazon, Amazon may know ' +
      'what you\'re looking at.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },

  // ── Social Tracking ───────────────────────────────────────────────────
  {
    vendor: 'Facebook / Meta Pixel',
    domain: '*.facebook.com',
    plain: 'Facebook tracks you on every site that has a Like button',
    description:
      'The Meta Pixel is embedded on millions of websites. Even if you\'re not on Facebook, ' +
      'Meta tracks every page you visit, what you buy, what you search for, and sends it all ' +
      'back to build your advertising profile. You don\'t need a Facebook account for this to happen.',
    category: 'Social',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'TikTok Analytics',
    domain: '*.tiktok.com',
    plain: 'TikTok monitors your activity across the web',
    description:
      'TikTok\'s analytics pixel tracks your activity on external websites, not just in the app. ' +
      'This data is collected on servers controlled by ByteDance and has been the subject of ' +
      'multiple government investigations over data sovereignty concerns.',
    category: 'Social',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'LinkedIn Insight Tag',
    domain: '*.linkedin.com',
    plain: 'LinkedIn tracks which websites you visit after seeing their ads',
    description:
      'The LinkedIn Insight Tag is embedded on business websites to track visitors and report ' +
      'back to LinkedIn which companies\' employees are looking at which sites. ' +
      'LinkedIn then uses this to target you with ads in your feed.',
    category: 'Social',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Snapchat Pixel',
    domain: '*.snapchat.com',
    plain: 'Snapchat tracks purchases and behavior on other websites',
    description:
      'The Snapchat Pixel lets advertisers see if you bought something after seeing a Snapchat ad. ' +
      'It reports your activity on external websites back to Snap Inc.',
    category: 'Social',
    risk: 'high',
    safeToBlock: true,
  },

  // ── Analytics ─────────────────────────────────────────────────────────
  {
    vendor: 'Google Analytics',
    domain: '*.google-analytics.com',
    plain: 'Reports every page you visit to Google',
    description:
      'Google Analytics is the most widely deployed tracker on the internet — it\'s on over ' +
      '55% of all websites. Every page you visit that uses it sends your IP address, browser, ' +
      'location, and behavior data to Google. Google uses this data across its advertising products.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Google Tag Manager',
    domain: '*.googletagmanager.com',
    plain: 'A container that loads other trackers onto websites',
    description:
      'Google Tag Manager is a tool that lets websites load many trackers at once. Blocking it ' +
      'can prevent dozens of other trackers from loading, but may also break some website features. ' +
      'It also reports usage data back to Google.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Mixpanel',
    domain: '*.mixpanel.com',
    plain: 'Records every click and action you take inside an app',
    description:
      'Mixpanel tracks every button click, feature use, and action you take inside apps and websites. ' +
      'It creates a detailed event log tied to your user ID. Companies use this to understand ' +
      'exactly how you use their product.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Amplitude',
    domain: '*.amplitude.com',
    plain: 'Logs every action you take with a timestamp',
    description:
      'Amplitude records detailed event streams of user behavior — what you clicked, when, ' +
      'in what order, for how long. This creates a detailed behavioral profile stored on Amplitude\'s servers.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Segment',
    domain: '*.segment.io',
    plain: 'Collects your data and routes it to dozens of other companies',
    description:
      'Segment acts as a data pipeline. It collects your behavior on a website or app and then ' +
      'automatically forwards it to other analytics, advertising, and marketing tools ' +
      '— often 10–30 different companies at once.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },

  // ── Microsoft ─────────────────────────────────────────────────────────
  {
    vendor: 'Microsoft Telemetry',
    domain: 'vortex.data.microsoft.com',
    plain: 'Windows sends usage reports to Microsoft',
    description:
      'Windows collects diagnostic and usage data and sends it to Microsoft through this endpoint. ' +
      'This includes information about installed software, hardware configuration, and how you use Windows. ' +
      'The amount of data depends on your privacy settings.',
    category: 'Microsoft',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Azure Application Insights',
    domain: '*.applicationinsights.io',
    plain: 'Web apps send your activity to Microsoft\'s cloud',
    description:
      'Application Insights is a developer monitoring tool embedded in many web apps. ' +
      'It reports performance data, errors, and user behavior back to Microsoft Azure. ' +
      'Users of those apps are tracked without their explicit knowledge.',
    category: 'Microsoft',
    risk: 'high',
    safeToBlock: true,
  },

  // ── Aggressive / Spyware ─────────────────────────────────────────────
  {
    vendor: 'Comscore (scorecardresearch)',
    domain: '*.scorecardresearch.com',
    plain: 'Surveillance company that builds audience profiles',
    description:
      'Scorecardresearch is operated by Comscore, an audience measurement company. ' +
      'It silently tracks your browsing across participating sites to build audience ' +
      'demographic reports that are sold to media companies and advertisers.',
    category: 'Analytics',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Quantcast',
    domain: '*.quantserve.com',
    plain: 'Tracks and sells your browsing profile to publishers',
    description:
      'Quantcast tracks your browsing across thousands of sites to infer your demographics, ' +
      'interests, and intent. This information is packaged and sold to publishers and advertisers ' +
      'as audience data.',
    category: 'Analytics',
    risk: 'critical',
    safeToBlock: true,
  },

  // ── Crash Reporting (lower risk, context matters) ─────────────────────
  {
    vendor: 'Sentry',
    domain: '*.sentry.io',
    plain: 'Sends error reports when an app crashes',
    description:
      'Sentry is a crash reporting tool that developers use to fix bugs. When an app crashes ' +
      'or encounters an error, it sends a report to Sentry including technical details. ' +
      'These reports may include some user context depending on how the app is configured.',
    category: 'Crash Reporting',
    risk: 'low',
    safeToBlock: false,
  },
  {
    vendor: 'Firebase Crashlytics',
    domain: 'crashlyticsreports-pa.googleapis.com',
    plain: 'Google\'s crash reporter for mobile apps',
    description:
      'Crashlytics sends crash reports from mobile apps to Google\'s Firebase platform. ' +
      'It helps developers fix app crashes. Reports include device info and app state at ' +
      'time of crash, but not personal content.',
    category: 'Crash Reporting',
    risk: 'low',
    safeToBlock: false,
  },
];

export const TRACKER_STATS = {
  total: TRACKERS.length,
  critical: TRACKERS.filter(t => t.risk === 'critical').length,
  high:     TRACKERS.filter(t => t.risk === 'high').length,
  safeToBlock: TRACKERS.filter(t => t.safeToBlock).length,
};
