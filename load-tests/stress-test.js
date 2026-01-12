// HSIP Phase 2 - Stress Test
// Pushes the system beyond normal limits to find breaking points

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const successCounter = new Counter('successes');

export const options = {
  stages: [
    { duration: '1m', target: 100 },   // Ramp up to 100 users
    { duration: '2m', target: 100 },   // Hold at 100
    { duration: '1m', target: 200 },   // Ramp up to 200 users
    { duration: '2m', target: 200 },   // Hold at 200
    { duration: '1m', target: 500 },   // Ramp up to 500 users
    { duration: '3m', target: 500 },   // Hold at 500 (stress level)
    { duration: '1m', target: 1000 },  // Ramp up to 1000 users
    { duration: '2m', target: 1000 },  // Hold at 1000 (breaking point)
    { duration: '2m', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<2000'], // 99% under 2s during stress
    http_req_failed: ['rate<0.10'],    // Allow 10% error rate under stress
    errors: ['rate<0.20'],             // Allow 20% custom errors under stress
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
  const token = login();
  return { token };
}

export default function (data) {
  const token = data.token;
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${token}`,
  };

  // Stress test with rapid-fire requests
  const batchSize = 5;
  const responses = http.batch([
    ['GET', `${BASE_URL}/health`, null, {}],
    ['POST', `${BASE_URL}/api/v2/messages/send`, JSON.stringify({
      recipient: `user${__VU}@example.com`,
      message_type: 'transaction_alert',
      content: { amount: '$100.00', merchant: 'Test Store' },
    }), { headers }],
    ['POST', `${BASE_URL}/api/v2/consent/grant`, JSON.stringify({
      user_id: `stress_user_${__VU}_${Date.now()}`,
      sender_domain: 'chase.com',
      allowed_message_types: ['transaction_alert'],
      duration_days: 30,
    }), { headers }],
  ]);

  // Check batch responses
  responses.forEach((res, i) => {
    const success = check(res, {
      'status is 2xx': (r) => r.status >= 200 && r.status < 300,
    });

    if (success) {
      successCounter.add(1);
    } else {
      errorRate.add(1);
    }
  });

  sleep(0.5); // Minimal sleep for stress
}
