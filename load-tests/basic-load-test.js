// HSIP Phase 2 - Basic Load Test
// Tests API performance under normal load

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Ramp up to 10 users
    { duration: '1m', target: 10 },    // Stay at 10 users
    { duration: '30s', target: 50 },   // Ramp up to 50 users
    { duration: '2m', target: 50 },    // Stay at 50 users
    { duration: '30s', target: 100 },  // Ramp up to 100 users
    { duration: '2m', target: 100 },   // Stay at 100 users
    { duration: '30s', target: 0 },    // Ramp down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'], // 95% of requests must complete below 500ms
    http_req_failed: ['rate<0.01'],   // Error rate must be below 1%
    errors: ['rate<0.05'],            // Custom error rate must be below 5%
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8787';

// Login and get JWT token
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

  check(loginRes, {
    'login status is 200': (r) => r.status === 200,
    'login returns access_token': (r) => JSON.parse(r.body).access_token !== undefined,
  });

  if (loginRes.status === 200) {
    return JSON.parse(loginRes.body).access_token;
  }
  return null;
}

export function setup() {
  // Get auth token for use in tests
  const token = login();
  return { token };
}

export default function (data) {
  const token = data.token;
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${token}`,
  };

  // Test 1: Health check
  const healthRes = http.get(`${BASE_URL}/health`);
  check(healthRes, {
    'health check status is 200': (r) => r.status === 200,
    'health check is healthy': (r) => JSON.parse(r.body).status === 'healthy',
  }) || errorRate.add(1);

  sleep(1);

  // Test 2: Send message
  const sendRes = http.post(
    `${BASE_URL}/api/v2/messages/send`,
    JSON.stringify({
      recipient: 'user@example.com',
      message_type: 'transaction_alert',
      content: {
        amount: '$50.00',
        merchant: 'Coffee Shop',
      },
    }),
    { headers }
  );

  check(sendRes, {
    'send message status is 200': (r) => r.status === 200,
    'send message returns message_id': (r) => {
      const body = JSON.parse(r.body);
      return body.message_id && body.message_id.startsWith('msg_');
    },
  }) || errorRate.add(1);

  sleep(1);

  // Test 3: Grant consent
  const consentRes = http.post(
    `${BASE_URL}/api/v2/consent/grant`,
    JSON.stringify({
      user_id: `user_${__VU}`,
      sender_domain: 'chase.com',
      allowed_message_types: ['transaction_alert', 'security_alert'],
      duration_days: 30,
      max_messages_per_day: 10,
    }),
    { headers }
  );

  check(consentRes, {
    'grant consent status is 200': (r) => r.status === 200,
    'grant consent returns policy_id': (r) => {
      const body = JSON.parse(r.body);
      return body.policy_id && body.policy_id.startsWith('policy_');
    },
  }) || errorRate.add(1);

  sleep(2);
}

export function teardown(data) {
  console.log('Load test completed');
}
