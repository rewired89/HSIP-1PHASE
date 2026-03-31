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
  'Data Broker',
  'Email Tracking',
  'Fingerprinting',
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
  {
    vendor: 'LogRocket',
    domain: '*.logrocket.com',
    plain: 'Records sessions and console logs from your browser',
    description:
      'LogRocket replays your entire browser session including network requests, console errors, ' +
      'and DOM changes. It can capture what you see on screen and data passing through the page.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Mouseflow',
    domain: '*.mouseflow.com',
    plain: 'Tracks your mouse movements and form inputs',
    description:
      'Mouseflow records mouse movements, clicks, scrolling, and form interactions. ' +
      'It generates heatmaps and session replays that show every action you took on a page.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Lucky Orange',
    domain: '*.luckyorange.com',
    plain: 'Live visitor recording and heatmap tool',
    description:
      'Lucky Orange gives website owners a live view of what visitors are doing in real time. ' +
      'It records sessions, tracks clicks and hovers, and can capture form field data.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Inspectlet',
    domain: '*.inspectlet.com',
    plain: 'Records everything you do on a website',
    description:
      'Inspectlet records every mouse move, scroll, click, and keystroke on websites that use it, ' +
      'creating a video of your visit that site owners can replay.',
    category: 'Session Recording',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Smartlook',
    domain: '*.smartlook.com',
    plain: 'Session recording for web and mobile apps',
    description:
      'Smartlook records your interactions on websites and mobile apps, including taps, swipes, ' +
      'scrolls, and rage clicks. Session replays are stored and shared with site owners.',
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
  {
    vendor: 'Outbrain',
    domain: '*.outbrain.com',
    plain: 'Sponsored content network that tracks reading habits',
    description:
      'Outbrain shows "recommended" articles on news sites and tracks which content you click, ' +
      'scroll past, and read. This data is used to build an interest profile for ad targeting.',
    category: 'Ad Network',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Taboola',
    domain: '*.taboola.com',
    plain: 'Clickbait ad network that tracks what you read',
    description:
      'Taboola powers the "around the web" sponsored articles at the bottom of news sites. ' +
      'It tracks which stories you read and click on across all Taboola-powered sites.',
    category: 'Ad Network',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Rubicon Project / Magnite',
    domain: '*.rubiconproject.com',
    plain: 'Ad auction platform that bids on your attention',
    description:
      'Magnite (formerly Rubicon) is a supply-side platform that runs real-time ad auctions ' +
      'as you load a page. Your browsing profile is shared with hundreds of bidders in milliseconds.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'OpenX',
    domain: '*.openx.net',
    plain: 'Ad exchange that auctions your profile to advertisers',
    description:
      'OpenX is a programmatic ad exchange. When you visit a page using OpenX, your browser ' +
      'profile is shared with advertisers who bid to show you ads.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'PubMatic',
    domain: '*.pubmatic.com',
    plain: 'Advertising platform tracking your browsing profile',
    description:
      'PubMatic connects publishers with advertisers using your browsing data. It sets tracking ' +
      'cookies to build audience profiles that are monetized through real-time bidding.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Index Exchange',
    domain: '*.casalemedia.com',
    plain: 'Programmatic ad marketplace using your data',
    description:
      'Index Exchange is a programmatic ad marketplace. It tracks users across sites to sell ' +
      'targeted advertising inventory. Your browsing data is shared with their advertiser partners.',
    category: 'Ad Network',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'ShareThis',
    domain: '*.sharethis.com',
    plain: 'Social sharing buttons that also track your behaviour',
    description:
      'ShareThis provides those social sharing buttons on articles. While doing so, it tracks ' +
      'what content you share, read, and engage with — selling this data to advertisers.',
    category: 'Ad Network',
    risk: 'high',
    safeToBlock: true,
  },

  // ── Social Tracking ───────────────────────────────────────────────────
  {
    vendor: 'Facebook / Meta Pixel',
    domain: 'connect.facebook.net',
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
    vendor: 'Facebook App Events',
    domain: 'graph.facebook.com',
    plain: 'Facebook\'s tracking API used by apps and websites',
    description:
      'graph.facebook.com is the endpoint apps and websites use to send your activity events ' +
      'directly to Facebook — purchases, signups, page views. Blocking this stops that reporting ' +
      'without affecting your ability to use Facebook itself.',
    category: 'Social',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'TikTok Analytics',
    domain: 'analytics.tiktok.com',
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
    domain: 'snap.licdn.com',
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
    domain: 'tr.snapchat.com',
    plain: 'Snapchat tracks purchases and behavior on other websites',
    description:
      'The Snapchat Pixel lets advertisers see if you bought something after seeing a Snapchat ad. ' +
      'It reports your activity on external websites back to Snap Inc.',
    category: 'Social',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Twitter / X Pixel',
    domain: '*.ads-twitter.com',
    plain: 'Twitter tracks conversions and web activity for ad targeting',
    description:
      'The Twitter (X) universal website tag tracks what you do after clicking a tweet or ad. ' +
      'It reports page views, purchases, and other events back to Twitter\'s ad platform.',
    category: 'Social',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Pinterest Tag',
    domain: 'ct.pinterest.com',
    plain: 'Pinterest tracks your purchases and page visits',
    description:
      'The Pinterest tag tracks what pages you visit and what you buy on websites that run it, ' +
      'linking this to your Pinterest activity to serve you targeted ads.',
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
  {
    vendor: 'Segment CDN',
    domain: '*.cdn.segment.com',
    plain: 'Loads Segment\'s tracking code onto websites',
    description:
      'This is the CDN that delivers Segment\'s analytics library to websites. ' +
      'Blocking it prevents Segment from loading and sending your data to their pipeline.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Heap Analytics',
    domain: '*.heapanalytics.com',
    plain: 'Automatically records every interaction without any setup',
    description:
      'Heap captures every click, tap, swipe, and page view automatically — it doesn\'t ' +
      'require developers to manually tag anything. This means it collects everything by default.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Intercom',
    domain: '*.intercom.io',
    plain: 'Chat widget that tracks your behavior and profile',
    description:
      'Intercom is a customer messaging platform. When embedded, it tracks your page views, ' +
      'session duration, location, and device info, and links this to your identity if you ' +
      'use the chat. This data is stored on Intercom\'s servers.',
    category: 'Analytics',
    risk: 'medium',
    safeToBlock: true,
  },
  {
    vendor: 'HubSpot',
    domain: '*.hubspot.com',
    plain: 'CRM that tracks your visits and links them to marketing campaigns',
    description:
      'HubSpot\'s tracking pixel links your visits to marketing emails you\'ve opened. ' +
      'If you click a link in a HubSpot email, your subsequent browsing on that site is tracked ' +
      'and tied to your contact record in the company\'s CRM.',
    category: 'Analytics',
    risk: 'medium',
    safeToBlock: true,
  },
  {
    vendor: 'Marketo',
    domain: '*.marketo.com',
    plain: 'Marketing automation that deanonymises your website visits',
    description:
      'Marketo tracks your visits and tries to link them to your email if you\'ve ever clicked ' +
      'a Marketo email. Once linked, your browsing history is tied to your identity in their database.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Pardot (Salesforce)',
    domain: '*.pardot.com',
    plain: 'Salesforce marketing tracker that follows leads across the web',
    description:
      'Pardot is Salesforce\'s B2B marketing platform. It tracks prospect visits to websites ' +
      'and scores leads based on which pages they view. If you\'ve clicked a Pardot email, ' +
      'your browsing is tied to your identity in their CRM.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Woopra',
    domain: '*.woopra.com',
    plain: 'Real-time customer journey analytics',
    description:
      'Woopra tracks individual users across multiple channels — website, email, mobile — ' +
      'and builds a unified profile of their complete journey. It connects anonymous visits ' +
      'to identified users.',
    category: 'Analytics',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Chartbeat',
    domain: '*.chartbeat.com',
    plain: 'Real-time news audience tracker',
    description:
      'Chartbeat is used by news sites to measure how long you spend reading articles. ' +
      'It reports your scroll depth, active reading time, and traffic source back to publishers.',
    category: 'Analytics',
    risk: 'medium',
    safeToBlock: true,
  },
  {
    vendor: 'Clicky',
    domain: '*.getclicky.com',
    plain: 'Website analytics that logs individual visitors',
    description:
      'Clicky provides real-time web analytics including individual visitor logs. ' +
      'Site owners can see each visitor\'s IP, location, pages viewed, and click paths.',
    category: 'Analytics',
    risk: 'medium',
    safeToBlock: true,
  },
  {
    vendor: 'StatCounter',
    domain: '*.statcounter.com',
    plain: 'Logs your IP address and browsing path on each visit',
    description:
      'StatCounter tracks individual visitors with detailed logs including IP address, ' +
      'browser, operating system, and the full sequence of pages visited.',
    category: 'Analytics',
    risk: 'medium',
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
    vendor: 'Microsoft Telemetry (Settings)',
    domain: 'settings-win.data.microsoft.com',
    plain: 'Sends Windows settings and configuration data to Microsoft',
    description:
      'This Microsoft endpoint receives telemetry related to Windows settings changes and ' +
      'system configuration. It is part of the Windows Connected User Experiences reporting pipeline.',
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
  {
    vendor: 'Bing Ads',
    domain: '*.bat.bing.com',
    plain: 'Microsoft\'s ad conversion tracker for websites',
    description:
      'The Bing Universal Event Tracking tag tracks what users do after clicking a Bing ad. ' +
      'It reports page views, purchases, and other events back to Microsoft Advertising.',
    category: 'Microsoft',
    risk: 'high',
    safeToBlock: true,
  },

  // ── Data Brokers ──────────────────────────────────────────────────────
  {
    vendor: 'Comscore / Scorecard Research',
    domain: '*.scorecardresearch.com',
    plain: 'Surveillance company that builds audience profiles',
    description:
      'Scorecardresearch is operated by Comscore, an audience measurement company. ' +
      'It silently tracks your browsing across participating sites to build audience ' +
      'demographic reports that are sold to media companies and advertisers.',
    category: 'Data Broker',
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
    category: 'Data Broker',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Nielsen',
    domain: '*.imrworldwide.com',
    plain: 'TV and internet audience measurement tracking',
    description:
      'Nielsen\'s digital measurement tag tracks your content consumption across the web ' +
      'to sell audience measurement data to media companies and advertisers.',
    category: 'Data Broker',
    risk: 'high',
    safeToBlock: true,
  },
  {
    vendor: 'Oracle BlueKai',
    domain: '*.bluekai.com',
    plain: 'Oracle\'s data marketplace that trades your profile',
    description:
      'BlueKai (now Oracle Data Cloud) aggregates data from hundreds of sources to create ' +
      'audience segments that are sold to advertisers. Your browsing, purchase, and demographic ' +
      'data are included in these segments.',
    category: 'Data Broker',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'Lotame',
    domain: '*.crwdcntrl.net',
    plain: 'Data marketplace that buys and sells your browsing profile',
    description:
      'Lotame is a data management platform that collects browsing data and sells it as audience ' +
      'segments to advertisers. It operates across a large network of publisher sites.',
    category: 'Data Broker',
    risk: 'critical',
    safeToBlock: true,
  },

  // ── Email Tracking ────────────────────────────────────────────────────
  {
    vendor: 'Mailchimp',
    domain: '*.list-manage.com',
    plain: 'Tracks whether you opened an email and which links you clicked',
    description:
      'Mailchimp embeds invisible 1x1 pixel images in emails. When your email client loads ' +
      'the image, Mailchimp records that you opened the email, when, and from where. ' +
      'Link clicks are also tracked.',
    category: 'Email Tracking',
    risk: 'medium',
    safeToBlock: false,
  },
  {
    vendor: 'Sendgrid',
    domain: '*.sendgrid.net',
    plain: 'Email delivery service that tracks open rates and clicks',
    description:
      'Sendgrid is used by companies to send bulk emails. It tracks whether recipients open ' +
      'emails (via tracking pixels) and which links they click, reporting back to the sender.',
    category: 'Email Tracking',
    risk: 'medium',
    safeToBlock: false,
  },

  // ── Fingerprinting ────────────────────────────────────────────────────
  {
    vendor: 'Cloudflare Insights',
    domain: '*.cloudflareinsights.com',
    plain: 'Website performance and analytics tool',
    description:
      'Cloudflare\'s Web Analytics collects basic visit data including browser type, ' +
      'country, and page performance. Unlike most analytics, Cloudflare claims not to build ' +
      'user profiles, but it still reports your visit data.',
    category: 'Analytics',
    risk: 'low',
    safeToBlock: true,
  },
  {
    vendor: 'Fingerprint.com (FingerprintJS Pro)',
    domain: '*.fpjs.io',
    plain: 'Identifies you across sessions without cookies',
    description:
      'FingerprintJS Pro builds a unique "fingerprint" of your browser based on your hardware, ' +
      'fonts, screen resolution, and other signals. This lets websites identify you even if you ' +
      'delete cookies, use private browsing, or switch networks.',
    category: 'Fingerprinting',
    risk: 'critical',
    safeToBlock: true,
  },
  {
    vendor: 'ThreatMetrix (LexisNexis)',
    domain: '*.online-metrix.net',
    plain: 'Device fingerprinting for fraud detection and identity',
    description:
      'ThreatMetrix (LexisNexis Risk Solutions) fingerprints your device for fraud detection. ' +
      'While marketed as a security tool, it builds a persistent identifier for your device ' +
      'that is shared across financial and insurance companies.',
    category: 'Fingerprinting',
    risk: 'high',
    safeToBlock: false,
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
  {
    vendor: 'Datadog RUM',
    domain: '*.datadoghq-browser-agent.com',
    plain: 'Monitors app performance and user sessions',
    description:
      'Datadog Real User Monitoring tracks page load times, errors, and user interactions. ' +
      'It can capture session replays and is primarily used for performance monitoring, ' +
      'though it does collect behavioral data.',
    category: 'Crash Reporting',
    risk: 'medium',
    safeToBlock: false,
  },
  {
    vendor: 'New Relic Browser',
    domain: '*.newrelic.com',
    plain: 'Application performance monitoring tool',
    description:
      'New Relic\'s browser agent monitors page performance and JavaScript errors. ' +
      'It reports timing data and error stacks back to New Relic servers.',
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

/**
 * Well-known first-party sites that users commonly look up.
 * These are the companies' own websites — not cross-site tracker services —
 * but they DO collect your data and/or operate tracking services listed above.
 *
 * Keys are bare domains (no www, no wildcard). Values include:
 *   name          — display name
 *   note          — plain-English explanation for non-tech users
 *   relatedTrackers — tracker domains from TRACKERS that this company operates
 */
export const FIRST_PARTY_BRANDS = {
  'google.com': {
    name: 'Google',
    note: "Google's own website is not a third-party tracker service embedded across other sites — but Google runs some of the most widespread tracking services on the internet, including Google Analytics (present on over 55% of all websites) and DoubleClick ads. Your activity on Google Search and other Google products feeds directly into their advertising profile of you.",
    relatedTrackers: ['google-analytics.com', 'doubleclick.net', 'googletagmanager.com'],
  },
  'openai.com': {
    name: 'OpenAI',
    note: "OpenAI operates ChatGPT and other AI services. It stores your conversations and usage data on its own servers and uses it to improve its models. It is not a third-party tracking service secretly embedded in other websites.",
    relatedTrackers: [],
  },
  'chatgpt.com': {
    name: 'ChatGPT (OpenAI)',
    note: "ChatGPT is OpenAI's AI assistant. Every conversation you have is stored on OpenAI's servers and used to train their models unless you opt out. It is not a tracker embedded across other websites.",
    relatedTrackers: [],
  },
  'amazon.com': {
    name: 'Amazon',
    note: "Amazon's shopping website collects everything you browse, search, add to cart, and buy — even things you don't purchase. Amazon also runs one of the world's largest advertising networks that tracks you across the web.",
    relatedTrackers: ['amazon-adsystem.com'],
  },
  'netflix.com': {
    name: 'Netflix',
    note: "Netflix tracks every video you watch, pause, rewind, search for, and rate. They use this to power recommendations and license data to studios. Netflix is not a cross-site tracker — it only collects data on its own platform.",
    relatedTrackers: [],
  },
  'youtube.com': {
    name: 'YouTube (Google)',
    note: "YouTube is owned by Google and tracks every video you watch, search for, pause, and click on. As a Google platform, your YouTube activity is linked to your broader Google advertising profile.",
    relatedTrackers: ['google-analytics.com', 'doubleclick.net'],
  },
  'instagram.com': {
    name: 'Instagram (Meta)',
    note: "Instagram is owned by Meta (Facebook) and tracks your posts, likes, stories, searches, and who you follow. Meta also runs tracking pixels that follow you across millions of other websites — even when you're not on Instagram.",
    relatedTrackers: ['facebook.com', 'fbcdn.net'],
  },
  'twitter.com': {
    name: 'Twitter / X',
    note: "Twitter/X tracks your tweets, reads, follows, and how long you look at each post. They also run ad conversion pixels embedded on external websites that track you after you see a tweet.",
    relatedTrackers: ['ads-twitter.com'],
  },
  'x.com': {
    name: 'X (formerly Twitter)',
    note: "X tracks your activity on their platform and also runs ad tracking pixels on external websites to measure whether you take action after seeing ads in your feed.",
    relatedTrackers: ['ads-twitter.com'],
  },
  'microsoft.com': {
    name: 'Microsoft',
    note: "Microsoft's website is not a cross-site tracker, but Microsoft operates multiple tracking services: Clarity (session recording on millions of sites), Xandr/AppNexus (ad exchange), Bing Ads, and Windows telemetry that phones home from your computer.",
    relatedTrackers: ['clarity.ms', 'adnxs.com', 'bat.bing.com', 'applicationinsights.io'],
  },
  'reddit.com': {
    name: 'Reddit',
    note: "Reddit tracks your posts, upvotes, subreddits, and reading patterns on their platform. They have expanded their advertising business significantly and now run conversion tracking for advertisers.",
    relatedTrackers: [],
  },
  'spotify.com': {
    name: 'Spotify',
    note: "Spotify tracks every song, podcast, and playlist you listen to, skip, or save. This data is used for recommendations and shared with music labels and advertisers for targeting.",
    relatedTrackers: [],
  },
  'apple.com': {
    name: 'Apple',
    note: "Apple positions itself as privacy-focused and does not operate a large cross-site tracking network. However, Apple collects device usage data, App Store behavior, and Siri queries. Apple Search Ads also tracks app installs.",
    relatedTrackers: [],
  },
  'whatsapp.com': {
    name: 'WhatsApp (Meta)',
    note: "WhatsApp is owned by Meta (Facebook). While your messages are end-to-end encrypted, Meta collects metadata about who you contact, how often, and when. This feeds into Meta's advertising profile of you.",
    relatedTrackers: ['facebook.com'],
  },
  'yahoo.com': {
    name: 'Yahoo',
    note: "Yahoo operates a search engine, news site, and advertising network. Verizon Media (now Oath/Yahoo) runs one of the largest ad exchanges and tracks users across the web for targeted advertising.",
    relatedTrackers: [],
  },
  'bing.com': {
    name: 'Bing (Microsoft)',
    note: "Bing is Microsoft's search engine. Every search is logged and used to build an advertising profile. Microsoft Advertising also tracks conversions on external websites through the Bing Ads pixel.",
    relatedTrackers: ['bat.bing.com', 'adnxs.com'],
  },
  'temu.com': {
    name: 'Temu',
    note: "Temu is a shopping app that has been scrutinized for extensive data collection practices. Its apps request unusually broad device permissions and send behavioral data back to servers in China.",
    relatedTrackers: [],
  },
  'shein.com': {
    name: 'SHEIN',
    note: "SHEIN is a fast-fashion retailer that collects browsing, purchase, and device data. Privacy researchers have flagged SHEIN's apps for excessive data collection and sending data to Chinese servers.",
    relatedTrackers: [],
  },
};
