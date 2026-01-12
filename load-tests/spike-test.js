// HSIP Phase 2 - Spike Test
// Tests how the system handles sudden traffic spikes

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '30s', target: 20 },    // Baseline load
    { duration: '10s', target: 500 },   // SPIKE: sudden jump to 500 users
    { duration: '1m', target: 500 },    // Hold spike
    { duration: '10s', target: 20 },    // Drop back to baseline
    { duration: '30s', target: 20 },    // Hold baseline
    { duration: '10s', target: 1000 },  // SPIKE: even bigger spike
    { duration: '1m', target: 1000 },   // Hold spike
    { duration: '10s', target: 0 },     // Drop to zero
  ],
  thresholds: {
    http_req_duration: ['p(95)<1000'], // 95% under 1s during spike
    http_req_failed: ['rate<0.15'],    // Allow 15% errors during spike
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8787';

function login() {
  const loginRes = http.post(
    `${BASE_URL}/api/v2/auth/login`,
    JSON.stringify({
      username: 'admin',
      password: 'admin',
    }),
    {
      headers: { 'Content-Type': 'application/json' },
    }
  );

  if (loginRes.status === 200) {
    return JSON.parse(loginRes.body).access_token;
  }
  return null;
}

export function setup() {
  return { token: login() };
}

export default function (data) {
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${data.token}`,
  };

  // Simulate realistic user behavior during spike
  const endpoints = [
    {
      name: 'health',
      request: () => http.get(`${BASE_URL}/health`),
    },
    {
      name: 'send_message',
      request: () => http.post(
        `${BASE_URL}/api/v2/messages/send`,
        JSON.stringify({
          recipient: `spike_user_${__VU}@example.com`,
          message_type: 'fraud_alert',
          content: { severity: 'high' },
        }),
        { headers }
      ),
    },
    {
      name: 'token_info',
      request: () => http.get(
        `${BASE_URL}/api/v2/auth/token/info`,
        { headers }
      ),
    },
  ];

  // Random endpoint selection
  const endpoint = endpoints[Math.floor(Math.random() * endpoints.length)];
  const res = endpoint.request();

  check(res, {
    [`${endpoint.name} status is 2xx`]: (r) => r.status >= 200 && r.status < 300,
  }) || errorRate.add(1);

  sleep(Math.random() * 2); // Random think time
}
