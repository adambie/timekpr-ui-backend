// Configuration
const API_BASE_URL = 'http://localhost:5000/api';

// State management
let currentUser = null;
let currentPage = 'login';

// Utility functions
function showAlert(message, type = 'info') {
    showNotification(message, type === 'danger' ? 'error' : type);
}

function showPage(pageId) {
    // Hide all pages
    document.querySelectorAll('[id$="-page"]').forEach(page => {
        page.classList.add('hidden');
    });
    
    // Show requested page
    document.getElementById(pageId).classList.remove('hidden');
    currentPage = pageId.replace('-page', '');
}


// Token management
function getToken() {
    return localStorage.getItem('jwt_token');
}

function setToken(token) {
    localStorage.setItem('jwt_token', token);
}

function removeToken() {
    localStorage.removeItem('jwt_token');
}

function isTokenExpired(token) {
    if (!token) return true;
    
    try {
        const payload = JSON.parse(atob(token.split('.')[1]));
        const currentTime = Math.floor(Date.now() / 1000);
        return payload.exp < currentTime;
    } catch (error) {
        return true;
    }
}

// API functions
async function apiCall(endpoint, method = 'GET', data = null) {
    const config = {
        method,
        headers: {
            'Content-Type': 'application/json',
        }
    };
    
    // Add Authorization header for authenticated endpoints (except login)
    if (endpoint !== '/login') {
        const token = getToken();
        if (token && !isTokenExpired(token)) {
            config.headers.Authorization = `Bearer ${token}`;
        }
    }
    
    if (data) {
        config.body = JSON.stringify(data);
    }
    
    try {
        const response = await fetch(`${API_BASE_URL}${endpoint}`, config);
        const result = await response.json();
        
        if (response.status === 401) {
            // Token expired or invalid - clear token and redirect to login
            removeToken();
            showLogin();
            return null;
        }
        
        return result;
    } catch (error) {
        console.error('API call failed:', error);
        showAlert('Connection error. Please check if the backend server is running.', 'danger');
        return null;
    }
}

// Authentication
async function login(username, password) {
    const result = await apiCall('/login', 'POST', { username, password });
    
    if (result && result.success && result.token) {
        setToken(result.token);
        currentUser = username;
        showDashboard();
        return true;
    } else {
        return result ? result.message : 'Login failed';
    }
}

async function logout() {
    // Clear the JWT token
    removeToken();
    currentUser = null;
    showLogin();
}

function showLogin() {
    showPage('login-page');
    
    // Clear login form fields
    document.getElementById('username').value = '';
    document.getElementById('password').value = '';
    
    // Hide any error messages
    document.getElementById('login-error').style.display = 'none';
}

// Dashboard
async function showDashboard() {
    showPage('dashboard-page');
    
    // Check SSH status first
    const sshStatus = await apiCall('/ssh-status');
    if (sshStatus && sshStatus.success && !sshStatus.ssh_key_exists) {
        showNotification(sshStatus.message, 'warning');
    }
    
    const result = await apiCall('/dashboard');
    const content = document.getElementById('dashboard-content');
    
    if (result && result.success) {
        content.innerHTML = renderDashboard(result.users, sshStatus);
        
        // Initialize charts for each user
        if (result.users && result.users.length > 0) {
            result.users.forEach(user => {
                setTimeout(async () => await initializeChart(user), 100);
            });
        }
    } else {
        content.innerHTML = '<div class="alert alert-danger">Failed to load dashboard data</div>';
    }
}

async function initializeChart(user) {
    const canvas = document.getElementById(`chart-${user.id}`);
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    
    // Get real usage data from API
    const result = await apiCall(`/user/${user.id}/usage`);
    
    let labels = [];
    let values = [];
    
    if (result && result.success && result.data) {
        // Process the new API format with actual dates
        result.data.forEach(dayData => {
            const date = new Date(dayData.date);
            const isToday = date.toDateString() === new Date().toDateString();
            const label = isToday ? 'Today' : date.toLocaleDateString('en', { weekday: 'short', day: 'numeric' });
            labels.push(label);
            values.push(dayData.hours);
        });
    } else {
        // If no data available, show 7 days with zeros
        const today = new Date();
        for (let i = 6; i >= 0; i--) {
            const date = new Date(today);
            date.setDate(date.getDate() - i);
            const isToday = i === 6;
            const label = isToday ? 'Today' : date.toLocaleDateString('en', { weekday: 'short', day: 'numeric' });
            labels.push(label);
            values.push(0);
        }
    }
    
    // Calculate appropriate Y-axis scale
    const maxValue = Math.max(...values, 1);
    const yAxisMax = Math.ceil(maxValue);
    const stepSize = yAxisMax <= 4 ? 0.5 : 1;
    
    new Chart(ctx, {
        type: 'bar',
        data: {
            labels: labels,
            datasets: [{
                label: 'Hours Used',
                data: values,
                backgroundColor: function(context) {
                    const index = context.dataIndex;
                    const weekday = labels[index];
                    if (weekday === 'Sat' || weekday === 'Sun') {
                        return 'rgb(245 158 11 / 0.6)';
                    }
                    return 'rgb(59 130 246 / 0.6)';
                },
                borderColor: function(context) {
                    const index = context.dataIndex;
                    const weekday = labels[index];
                    if (weekday === 'Sat' || weekday === 'Sun') {
                        return 'rgb(245 158 11)';
                    }
                    return 'rgb(59 130 246)';
                },
                borderWidth: 2,
                borderRadius: 6,
                borderSkipped: false
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: {
                intersect: false,
                mode: 'index'
            },
            scales: {
                y: {
                    beginAtZero: true,
                    max: yAxisMax,
                    ticks: {
                        stepSize: stepSize,
                        color: 'rgb(100 116 139)',
                        font: { size: 11 },
                        callback: function(value) {
                            if (value === 0) return '0h';
                            if (value < 1) {
                                const minutes = Math.round(value * 60);
                                return `${minutes}m`;
                            }
                            if (value % 1 === 0) {
                                return `${value}h`;
                            }
                            const hours = Math.floor(value);
                            const minutes = Math.round((value % 1) * 60);
                            return `${hours}h${minutes}m`;
                        }
                    },
                    title: {
                        display: true,
                        text: 'Hours',
                        color: 'rgb(100 116 139)',
                        font: { size: 12, weight: '500' }
                    },
                    grid: {
                        color: 'rgb(226 232 240 / 0.5)',
                    }
                },
                x: {
                    grid: { display: false },
                    ticks: {
                        color: function(context) {
                            const weekday = labels[context.index];
                            if (weekday === 'Sat' || weekday === 'Sun') {
                                return 'rgb(245 158 11)';
                            }
                            return 'rgb(100 116 139)';
                        },
                        font: { size: 12, weight: '600' }
                    }
                }
            },
            plugins: {
                legend: { display: false },
                tooltip: {
                    callbacks: {
                        label: function(context) {
                            const hours = context.parsed.y;
                            const minutes = Math.round((hours % 1) * 60);
                            const wholeHours = Math.floor(hours);
                            
                            if (minutes === 0) {
                                return `${wholeHours}h 0m`;
                            } else {
                                return `${wholeHours}h ${minutes}m`;
                            }
                        }
                    }
                }
            }
        }
    });
}

function renderDashboard(users, sshStatus = null) {
    let content = '';
    
    // Show SSH warning if keys are missing
    if (sshStatus && sshStatus.success && !sshStatus.ssh_key_exists) {
        content += `
            <div class="alert alert-warning" style="grid-column: 1 / -1; margin-bottom: var(--space-6);">
                <svg width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566zM8 5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995A.905.905 0 0 1 8 5zm.002 6a1 1 0 1 1 0 2 1 1 0 0 1 0-2z"/>
                </svg>
                ${sshStatus.message}
            </div>
        `;
    }
    
    if (!users || users.length === 0) {
        content += '<div class="empty-state" style="grid-column: 1 / -1;"><h2>No Users Available</h2><p>No valid users have been added yet. Go to the Admin Panel to add and manage users.</p><button class="btn btn-primary btn-lg" onclick="showAdmin()">Go to Admin Panel</button></div>';
        return content;
    }
    
    content += users.map(user => `
        <div class="user-card">
            <div class="user-card-header">
                <div class="user-info">
                    <h2 class="user-name">${user.username}</h2>
                    <p class="user-location">${user.system_ip}</p>
                </div>
            </div>
            <div class="user-card-body">
                <div class="user-stats">
                    <div class="stat-item">
                        <div class="stat-label">Time Left Today</div>
                        <div class="stat-value">${user.time_left || 'Unknown'}</div>
                    </div>
                    <div class="stat-item">
                        <div class="stat-label">Last Updated</div>
                        <div class="stat-value">${user.last_checked || 'Never'}</div>
                    </div>
                </div>
                
                ${user.pending_adjustment ? `
                <div class="badge badge-warning" style="margin-bottom: var(--space-4);">
                    <svg width="12" height="12" fill="currentColor" viewBox="0 0 16 16">
                        <path d="M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566zM8 5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995A.905.905 0 0 1 8 5zm.002 6a1 1 0 1 1 0 2 1 1 0 0 1 0-2z"/>
                    </svg>
                    Pending Time: ${user.pending_adjustment}
                </div>
                ` : ''}
                ${user.pending_schedule ? `
                <div class="badge badge-warning" style="margin-bottom: var(--space-4);">
                    <svg width="12" height="12" fill="currentColor" viewBox="0 0 16 16">
                        <path d="M8 3.5a.5.5 0 0 0-1 0V9a.5.5 0 0 0 .252.434l3.5 2a.5.5 0 0 0 .496-.868L8 8.71V3.5z"/>
                        <path d="M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16zm7-8A7 7 0 1 1 1 8a7 7 0 0 1 14 0z"/>
                    </svg>
                    Pending Schedule
                </div>
                ` : ''}
                
                <div class="user-actions">
                    <button class="btn btn-primary" onclick="openTimeAdjustModal(${user.id}, '${user.username}')">Adjust Time</button>
                    <button class="btn btn-secondary" onclick="showSchedule(${user.id})">Schedule</button>
                </div>
                
                <div class="user-chart">
                    <h4 class="chart-title">Usage This Week</h4>
                    <div class="chart-container">
                        <canvas id="chart-${user.id}"></canvas>
                    </div>
                </div>
            </div>
        </div>
    `).join('');
    
    return content;
}


// Admin
async function showAdmin() {
    showPage('admin-page');
    
    const result = await apiCall('/admin');
    const content = document.getElementById('admin-content');
    
    if (result && result.success) {
        content.innerHTML = renderAdmin(result.users);
    } else {
        content.innerHTML = '<div class="alert alert-danger">Failed to load admin data</div>';
    }
}

function renderAdmin(users) {
    if (!users || users.length === 0) {
        return '<div class="alert alert-info">No users found.</div>';
    }
    
    return `
        <div class="card">
            <div class="card-header">
                <h5>Managed Users</h5>
            </div>
            <div class="card-body">
                <div class="table-responsive">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>Username</th>
                                <th>System IP</th>
                                <th>Status</th>
                                <th>Last Checked</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${users.map(user => `
                                <tr>
                                    <td>${user.username}</td>
                                    <td>${user.system_ip}</td>
                                    <td>
                                        <span class="badge bg-${user.is_valid ? 'success' : 'danger'}">
                                            ${user.is_valid ? 'Valid' : 'Invalid'}
                                        </span>
                                    </td>
                                    <td>${user.last_checked}</td>
                                    <td>
                                        <button class="btn btn-sm btn-primary" onclick="validateUser(${user.id})">Validate</button>
                                        <button class="btn btn-sm btn-danger" onclick="deleteUser(${user.id}, '${user.username}')">Delete</button>
                                    </td>
                                </tr>
                            `).join('')}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    `;
}

async function addUser(username, systemIp) {
    const result = await apiCall('/users/add', 'POST', {
        username,
        system_ip: systemIp
    });
    
    if (result && result.success) {
        showAlert(result.message, 'success');
        document.getElementById('add-user-form').reset();
        setTimeout(showAdmin, 1000);
    } else {
        showAlert(result ? result.message : 'Failed to add user', 'danger');
    }
}

async function validateUser(userId) {
    const result = await apiCall(`/users/validate/${userId}`);
    
    if (result && result.success) {
        showAlert(result.message, 'success');
        setTimeout(showAdmin, 1000);
    } else {
        showAlert(result ? result.message : 'Failed to validate user', 'danger');
    }
}

async function deleteUser(userId, username) {
    if (confirm(`Are you sure you want to delete user "${username}"?`)) {
        const result = await apiCall(`/users/delete/${userId}`, 'POST');
        
        if (result && result.success) {
            showAlert(result.message, 'success');
            setTimeout(showAdmin, 1000);
        } else {
            showAlert(result ? result.message : 'Failed to delete user', 'danger');
        }
    }
}

// Settings
async function showSettings() {
    showPage('settings-page');
}

async function changePassword(currentPassword, newPassword, confirmPassword) {
    if (newPassword !== confirmPassword) {
        showAlert('New passwords do not match', 'danger');
        return;
    }
    
    const result = await apiCall('/change-password', 'POST', {
        current_password: currentPassword,
        new_password: newPassword,
        confirm_password: confirmPassword
    });
    
    const messageDiv = document.getElementById('password-message');
    
    if (result && result.success) {
        messageDiv.innerHTML = '<div class="alert alert-success">Password changed successfully</div>';
        document.getElementById('password-form').reset();
    } else {
        messageDiv.innerHTML = `<div class="alert alert-danger">${result ? result.message : 'Failed to change password'}</div>`;
    }
}

// Event listeners
// Check if user is already logged in
async function checkSession() {
    const token = getToken();
    if (!token || isTokenExpired(token)) {
        removeToken();
        return false;
    }
    
    // Token exists and is valid, try to access dashboard
    const result = await apiCall('/dashboard');
    if (result && result.success) {
        currentUser = 'admin'; // We know it's admin since login succeeded
        showDashboard();
        return true;
    } else {
        // Token might be invalid on server side
        removeToken();
        return false;
    }
}

document.addEventListener('DOMContentLoaded', async function() {
    // Initialize theme
    initializeTheme();
    
    // Check if already logged in
    const isLoggedIn = await checkSession();
    if (isLoggedIn) {
        return; // Already logged in, dashboard is shown
    }
    // Login form
    document.getElementById('login-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const username = document.getElementById('username').value;
        const password = document.getElementById('password').value;
        const errorDiv = document.getElementById('login-error');
        
        const result = await login(username, password);
        
        if (result !== true) {
            errorDiv.textContent = result;
            errorDiv.style.display = 'block';
        } else {
            errorDiv.style.display = 'none';
        }
    });
    
    // Add user form
    document.getElementById('add-user-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const username = document.getElementById('new-username').value;
        const systemIp = document.getElementById('new-system-ip').value;
        
        await addUser(username, systemIp);
    });
    
    // Password change form
    document.getElementById('password-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const currentPassword = document.getElementById('current-password').value;
        const newPassword = document.getElementById('new-password').value;
        const confirmPassword = document.getElementById('confirm-password').value;
        
        await changePassword(currentPassword, newPassword, confirmPassword);
    });
    
    // Start with login page
    showLogin();
});

// Time adjustment modal variables
let currentUserId = null;
let currentUsername = '';
let timeAdjustment = 0; // in minutes

// Function to show notifications
function showNotification(message, type = 'success') {
    const toast = document.getElementById('notification-toast');
    const messageEl = toast.querySelector('.toast-message');
    const iconEl = toast.querySelector('.toast-icon path');
    
    // Set message
    messageEl.textContent = message;
    
    // Set type and icon
    toast.className = `notification-toast ${type}`;
    if (type === 'success') {
        iconEl.setAttribute('d', 'M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zm-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.061L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z');
    } else if (type === 'error') {
        iconEl.setAttribute('d', 'M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z');
    } else if (type === 'warning') {
        iconEl.setAttribute('d', 'M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566zM8 5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995A.905.905 0 0 1 8 5zm.002 6a1 1 0 1 1 0 2 1 1 0 0 1 0-2z');
    } else if (type === 'info') {
        iconEl.setAttribute('d', 'M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16zm.93-9.412-1 4.705c-.07.34.029.533.304.533.194 0 .487-.07.686-.246l-.088.416c-.287.346-.92.598-1.465.598-.703 0-1.002-.422-.808-1.319l.738-3.468c.064-.293.006-.399-.287-.47l-.451-.081.082-.381 2.29-.287zM8 5.5a1 1 0 1 1 0-2 1 1 0 0 1 0 2z');
    }
    
    // Show toast
    toast.classList.add('show');
    
    // Hide after 2 seconds
    setTimeout(() => {
        toast.classList.remove('show');
    }, 2000);
}

// Function to open the time adjustment modal
function openTimeAdjustModal(userId, username) {
    currentUserId = userId;
    currentUsername = username;
    document.getElementById('modalUsername').textContent = username;
    resetTime();
    document.getElementById('timeAdjustModal').style.display = 'flex';
}

// Function to close the modal
function closeTimeAdjustModal() {
    document.getElementById('timeAdjustModal').style.display = 'none';
    document.getElementById('modalStatus').textContent = '';
    document.getElementById('modalStatus').className = '';
}

// Function to adjust time
function adjustTime(minutes) {
    timeAdjustment += minutes;
    updateTimeDisplay();
}

// Function to reset time adjustment
function resetTime() {
    timeAdjustment = 0;
    updateTimeDisplay();
}

// Function to update the time display
function updateTimeDisplay() {
    const display = document.getElementById('timeAdjustment');
    display.textContent = timeAdjustment;
    
    // Update color based on value
    if (timeAdjustment > 0) {
        display.style.color = 'var(--success)';
    } else if (timeAdjustment < 0) {
        display.style.color = 'var(--danger)';
    } else {
        display.style.color = 'var(--text-primary)';
    }
}

// Function to submit the time adjustment
async function submitTimeAdjustment() {
    if (timeAdjustment === 0) {
        showNotification('No time adjustment specified', 'error');
        return;
    }
    
    const seconds = timeAdjustment * 60; // Convert minutes to seconds
    const operation = seconds > 0 ? '+' : '-';
    const absoluteSeconds = Math.abs(seconds);
    
    // Show loading status
    const statusEl = document.getElementById('modalStatus');
    statusEl.textContent = 'Applying changes...';
    statusEl.className = 'status-loading';
    
    const result = await apiCall('/modify-time', 'POST', {
        user_id: currentUserId,
        operation,
        seconds: absoluteSeconds
    });
    
    if (result && result.success) {
        statusEl.textContent = 'Success! Time adjusted successfully.';
        statusEl.className = 'status-success';
        
        // Refresh the dashboard after 1.5 seconds
        setTimeout(() => {
            closeTimeAdjustModal();
            showDashboard();
        }, 1500);
    } else {
        statusEl.textContent = `Error: ${result ? result.message : 'Failed to adjust time'}`;
        statusEl.className = 'status-error';
    }
}

// Function to show schedule
async function showSchedule(userId) {
    const result = await apiCall(`/schedule-sync-status/${userId}`);
    
    if (result && result.success) {
        const schedule = result.schedule || {
            hours: { monday: 0, tuesday: 0, wednesday: 0, thursday: 0, friday: 0, saturday: 0, sunday: 0 },
            intervals: { 
                monday: { start_time: '00:00', end_time: '23:59' },
                tuesday: { start_time: '00:00', end_time: '23:59' },
                wednesday: { start_time: '00:00', end_time: '23:59' },
                thursday: { start_time: '00:00', end_time: '23:59' },
                friday: { start_time: '00:00', end_time: '23:59' },
                saturday: { start_time: '00:00', end_time: '23:59' },
                sunday: { start_time: '00:00', end_time: '23:59' }
            }
        };
        
        // Get username from dashboard data
        const user = document.querySelector(`[onclick*="showSchedule(${userId})"]`)?.closest('.user-card')?.querySelector('.user-name')?.textContent || 'User';
        
        // Set modal username and user ID
        document.getElementById('scheduleModalUsername').textContent = user;
        document.getElementById('scheduleUserId').value = userId;
        
        // Populate schedule rows
        const scheduleRows = document.getElementById('schedule-rows');
        scheduleRows.innerHTML = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'].map((day, index) => {
            const dayKey = day.toLowerCase();
            const isWeekend = index >= 5;
            
            // Get time intervals from the schedule (using new simplified format)
            const intervals = schedule.intervals || {};
            const dayInterval = intervals[dayKey] || { start_time: '00:00', end_time: '23:59' };
            
            return `
                <div class="schedule-row" style="padding: var(--space-4) var(--space-6); display: grid; grid-template-columns: 120px 1fr 1fr; gap: var(--space-4); align-items: center; border-bottom: 1px solid var(--border-primary); transition: background-color var(--transition-fast); ${index === 6 ? 'border-bottom: none;' : ''}">
                    <div style="font-weight: 600; color: ${isWeekend ? 'var(--warning)' : 'var(--text-primary)'}; font-size: var(--font-size-base);">${day}</div>
                    
                    <!-- Daily Time Limit Column -->
                    <div style="display: flex; align-items: center; gap: var(--space-3);">
                        <div style="display: flex; align-items: center; gap: var(--space-2);">
                            <button type="button" onclick="adjustScheduleTime('${dayKey}', -0.25)" style="width: 32px; height: 32px; border: 1px solid var(--border-primary); background: var(--bg-elevated); color: var(--text-primary); border-radius: var(--radius-sm); display: flex; align-items: center; justify-content: center; cursor: pointer; font-weight: 600;">−</button>
                            <input type="number" 
                                   id="${dayKey}" 
                                   name="${dayKey}" 
                                   value="${schedule.hours[dayKey] || 0}" 
                                   min="0" 
                                   max="24" 
                                   step="0.25" 
                                   style="width: 80px; padding: var(--space-2) var(--space-3); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-primary); color: var(--text-primary); text-align: center; font-size: var(--font-size-base); font-weight: 500;">
                            <button type="button" onclick="adjustScheduleTime('${dayKey}', 0.25)" style="width: 32px; height: 32px; border: 1px solid var(--border-primary); background: var(--bg-elevated); color: var(--text-primary); border-radius: var(--radius-sm); display: flex; align-items: center; justify-content: center; cursor: pointer; font-weight: 600;">+</button>
                        </div>
                        <span style="font-size: var(--font-size-sm); color: var(--text-tertiary); min-width: 40px;">hours</span>
                    </div>
                    
                    <!-- Time Range Column -->
                    <div style="display: flex; align-items: center; gap: var(--space-2);">
                        <input type="time" 
                               id="${dayKey}_start_time" 
                               name="${dayKey}_start_time" 
                               value="${dayInterval.start_time}" 
                               style="padding: var(--space-2) var(--space-3); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-primary); color: var(--text-primary); font-size: var(--font-size-base);">
                        <span style="color: var(--text-tertiary); font-weight: 600;">to</span>
                        <input type="time" 
                               id="${dayKey}_end_time" 
                               name="${dayKey}_end_time" 
                               value="${dayInterval.end_time}" 
                               style="padding: var(--space-2) var(--space-3); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-primary); color: var(--text-primary); font-size: var(--font-size-base);">
                    </div>
                </div>
            `;
        }).join('');
        
        // Clear any previous status
        document.getElementById('scheduleModalStatus').innerHTML = '';
        
        // Show the modal
        document.getElementById('scheduleModal').classList.add('active');
    } else {
        showNotification('Failed to load schedule data', 'error');
    }
}

// Schedule helper functions
function adjustScheduleTime(dayId, change) {
    const input = document.getElementById(dayId);
    let currentValue = parseFloat(input.value) || 0;
    let newValue = Math.max(0, Math.min(24, currentValue + change));
    input.value = newValue;
}

function setWeekdays() {
    const hours = document.getElementById('bulk-hours').value;
    if (!hours) return;
    ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'].forEach(day => {
        document.getElementById(day).value = hours;
    });
}

function setWeekends() {
    const hours = document.getElementById('bulk-hours').value;
    if (!hours) return;
    ['saturday', 'sunday'].forEach(day => {
        document.getElementById(day).value = hours;
    });
}

function setAllDays() {
    const hours = document.getElementById('bulk-hours').value;
    if (!hours) return;
    ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].forEach(day => {
        document.getElementById(day).value = hours;
    });
}

function adjustBulkTime(change) {
    const input = document.getElementById('bulk-hours');
    let currentValue = parseFloat(input.value) || 0;
    let newValue = Math.max(0, Math.min(24, currentValue + change));
    input.value = newValue;
}

function setTimeRangeAll() {
    const startTime = document.getElementById('bulk-start-time').value;
    const endTime = document.getElementById('bulk-end-time').value;
    
    if (!startTime || !endTime) {
        showNotification('Please enter both start and end times', 'warning');
        return;
    }
    
    if (startTime >= endTime) {
        showNotification('Start time must be before end time', 'warning');
        return;
    }
    
    // Set all days
    ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].forEach(day => {
        document.getElementById(`${day}_start_time`).value = startTime;
        document.getElementById(`${day}_end_time`).value = endTime;
    });
    
    showNotification(`Time range ${startTime} - ${endTime} applied to all days`, 'success');
}

// Function to save schedule
function closeScheduleModal() {
    document.getElementById('scheduleModal').classList.remove('active');
    document.getElementById('scheduleModalStatus').innerHTML = '';
}

async function submitSchedule() {
    const userId = document.getElementById('scheduleUserId').value;
    const scheduleData = {
        user_id: parseInt(userId),
        monday: parseFloat(document.getElementById('monday').value || 0),
        tuesday: parseFloat(document.getElementById('tuesday').value || 0),
        wednesday: parseFloat(document.getElementById('wednesday').value || 0),
        thursday: parseFloat(document.getElementById('thursday').value || 0),
        friday: parseFloat(document.getElementById('friday').value || 0),
        saturday: parseFloat(document.getElementById('saturday').value || 0),
        sunday: parseFloat(document.getElementById('sunday').value || 0),
        
        // Time intervals for each day (simplified format)
        monday_start_time: document.getElementById('monday_start_time').value,
        monday_end_time: document.getElementById('monday_end_time').value,
        tuesday_start_time: document.getElementById('tuesday_start_time').value,
        tuesday_end_time: document.getElementById('tuesday_end_time').value,
        wednesday_start_time: document.getElementById('wednesday_start_time').value,
        wednesday_end_time: document.getElementById('wednesday_end_time').value,
        thursday_start_time: document.getElementById('thursday_start_time').value,
        thursday_end_time: document.getElementById('thursday_end_time').value,
        friday_start_time: document.getElementById('friday_start_time').value,
        friday_end_time: document.getElementById('friday_end_time').value,
        saturday_start_time: document.getElementById('saturday_start_time').value,
        saturday_end_time: document.getElementById('saturday_end_time').value,
        sunday_start_time: document.getElementById('sunday_start_time').value,
        sunday_end_time: document.getElementById('sunday_end_time').value
    };
    
    console.log('Saving schedule:', scheduleData);
    
    const statusEl = document.getElementById('scheduleModalStatus');
    statusEl.innerHTML = '<div class="spinner"></div> Saving schedule...';
    statusEl.className = 'text-info';
    
    const result = await apiCall('/schedule/update', 'POST', scheduleData);
    
    if (result && result.success) {
        statusEl.innerHTML = '✓ Schedule saved successfully';
        statusEl.className = 'text-success';
        setTimeout(() => {
            closeScheduleModal();
            showDashboard(); // Refresh dashboard
        }, 1000);
    } else {
        const message = result ? result.message : 'Failed to save schedule';
        statusEl.innerHTML = `✗ ${message}`;
        statusEl.className = 'text-danger';
    }
}

// Theme management
function toggleTheme() {
    const html = document.documentElement;
    const toggle = document.querySelector('.theme-toggle');
    const icon = toggle.querySelector('.theme-toggle-icon path');
    const text = toggle.querySelector('.theme-toggle-text');
    
    const currentTheme = html.getAttribute('data-theme');
    const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
    
    html.setAttribute('data-theme', newTheme);
    localStorage.setItem('theme', newTheme);
    
    if (newTheme === 'dark') {
        // Moon icon for dark mode
        icon.setAttribute('d', 'M6 .278a.768.768 0 0 1 .08.858 7.208 7.208 0 0 0-.878 3.46c0 4.021 3.278 7.277 7.318 7.277.527 0 1.04-.055 1.533-.16a.787.787 0 0 1 .81.316.733.733 0 0 1-.031.893A8.349 8.349 0 0 1 8.344 16C3.734 16 0 12.286 0 7.71 0 4.266 2.114 1.312 5.124.06A.752.752 0 0 1 6 .278z');
        text.textContent = 'Light';
    } else {
        // Sun icon for light mode
        icon.setAttribute('d', 'M8 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM8 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 0zm0 13a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 13zm8-5a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2a.5.5 0 0 1 .5.5zM3 8a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2A.5.5 0 0 1 3 8zm10.657-5.657a.5.5 0 0 1 0 .707l-1.414 1.415a.5.5 0 1 1-.707-.708l1.414-1.414a.5.5 0 0 1 .707 0zm-9.193 9.193a.5.5 0 0 1 0 .707L3.05 13.657a.5.5 0 0 1-.707-.707l1.414-1.414a.5.5 0 0 1 .707 0zm9.193 2.121a.5.5 0 0 1-.707 0l-1.414-1.414a.5.5 0 0 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .707zM4.464 4.465a.5.5 0 0 1-.707 0L2.343 3.05a.5.5 0 1 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .708z');
        text.textContent = 'Dark';
    }
}

// Weekly Schedule functionality
async function showWeeklySchedule() {
    showPage('weekly-schedule-page');
    
    // Load users and their schedules
    const usersResult = await apiCall('/dashboard');
    if (!usersResult || !usersResult.success) {
        showNotification('Failed to load user data', 'error');
        return;
    }
    
    const content = document.getElementById('weekly-schedule-content');
    const users = usersResult.users;
    
    if (users.length === 0) {
        content.innerHTML = `
            <div class="empty-state" style="grid-column: 1 / -1;">
                <svg width="48" height="48" fill="var(--text-muted)" viewBox="0 0 16 16" style="margin-bottom: var(--space-4);">
                    <path d="M15 14s1 0 1-1-1-4-5-4-5 3-5 4 1 1 1 1h8zm-7.978-1A.261.261 0 0 1 7 12.996c.001-.264.167-1.03.76-1.72C8.312 10.629 9.282 10 11 10c1.717 0 2.687.63 3.24 1.276.593.69.758 1.457.76 1.72l-.008.002a.274.274 0 0 1-.014.002H7.022zM11 7a2 2 0 1 0 0-4 2 2 0 0 0 0 4zm3-2a3 3 0 1 1-6 0 3 3 0 0 1 6 0zM6.936 9.28a5.88 5.88 0 0 0-1.23-.247A7.35 7.35 0 0 0 5 9c-4 0-5 3-5 4 0 .667.333 1 1 1h4.216A2.238 2.238 0 0 1 5 13c0-1.01.377-2.042 1.09-2.904.243-.294.526-.569.846-.816zM4.92 10A5.493 5.493 0 0 0 4 13H1c0-.26.164-1.03.76-1.724.545-.636 1.492-1.256 3.16-1.275zM1.5 5.5a3 3 0 1 1 6 0 3 3 0 0 1-6 0zm3-2a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/>
                </svg>
                <h2>No Users Available</h2>
                <p>No users have been added yet. Go to the Admin Panel to add and manage users.</p>
                <button class="btn btn-primary btn-lg" onclick="showAdmin()">Go to Admin Panel</button>
            </div>
        `;
        return;
    }
    
    // Load schedule data for each user
    const usersWithSchedules = await Promise.all(
        users.map(async (user) => {
            const scheduleResult = await apiCall(`/schedule-sync-status/${user.id}`);
            return {
                ...user,
                schedule: scheduleResult && scheduleResult.success ? scheduleResult.schedule : null,
                isScheduleSynced: scheduleResult && scheduleResult.success ? scheduleResult.is_synced : true,
                lastScheduleSync: scheduleResult && scheduleResult.success ? scheduleResult.last_synced : null,
                lastScheduleModified: scheduleResult && scheduleResult.success ? scheduleResult.last_modified : null
            };
        })
    );
    
    // Generate schedule cards for all users
    content.innerHTML = usersWithSchedules.map(user => {
        const schedule = user.schedule || {
            hours: { monday: 0, tuesday: 0, wednesday: 0, thursday: 0, friday: 0, saturday: 0, sunday: 0 },
            intervals: { 
                monday: { start_time: '00:00', end_time: '23:59' },
                tuesday: { start_time: '00:00', end_time: '23:59' },
                wednesday: { start_time: '00:00', end_time: '23:59' },
                thursday: { start_time: '00:00', end_time: '23:59' },
                friday: { start_time: '00:00', end_time: '23:59' },
                saturday: { start_time: '00:00', end_time: '23:59' },
                sunday: { start_time: '00:00', end_time: '23:59' }
            }
        };
        
        const syncBadge = user.pending_schedule || !user.isScheduleSynced ? `
            <div class="badge badge-warning" style="margin-bottom: var(--space-3);">
                <svg width="12" height="12" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566zM8 5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995A.905.905 0 0 1 8 5zm.002 6a1 1 0 1 1 0 2 1 1 0 0 1 0-2z"/>
                </svg>
                Not Synced
            </div>
        ` : `
            <div class="badge badge-success" style="margin-bottom: var(--space-3);">
                <svg width="12" height="12" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zm-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.061L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z"/>
                </svg>
                Synced
            </div>
        `;
        
        return `
            <div class="user-card">
                <div class="user-card-header">
                    <div class="user-info">
                        <h2 class="user-name">${user.username}</h2>
                        <p class="user-location">${user.system_ip}</p>
                    </div>
                    <div style="display: flex; flex-direction: column; gap: var(--space-2);">
                        ${syncBadge}
                        <button class="btn btn-sm btn-secondary" onclick="showSchedule(${user.id})">Details</button>
                    </div>
                </div>
                
                <div class="user-card-body">
                    <form id="schedule-form-${user.id}" onsubmit="submitScheduleForm(event, ${user.id})">
                        <div class="schedule-section">
                            <h3 class="schedule-section-title">Weekly Time Limits & Allowed Hours</h3>
                            <div style="margin-bottom: var(--space-4); padding: var(--space-3); background: var(--bg-secondary); border-radius: var(--radius-md); font-size: var(--font-size-sm);">
                                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: var(--space-3); text-align: center;">
                                    ${['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].map(day => {
                                        const dayShort = day.substring(0, 3);
                                        const startTime = schedule.intervals?.[day]?.start_time || '00:00';
                                        const endTime = schedule.intervals?.[day]?.end_time || '23:59';
                                        return `
                                            <div>
                                                <div><strong>${dayShort.charAt(0).toUpperCase() + dayShort.slice(1)}:</strong> ${schedule.hours[day]}h</div>
                                                <div style="font-size: var(--font-size-xs); color: var(--text-muted); margin-top: var(--space-1);">${startTime}-${endTime}</div>
                                            </div>
                                        `;
                                    }).join('')}
                                </div>
                            </div>
                            <div style="display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-4); flex-wrap: wrap;">
                                <div style="display: flex; align-items: center; gap: var(--space-2);">
                                    <label style="font-weight: 500; white-space: nowrap;">Quick hours (Mon-Fri):</label>
                                    <input type="number" id="weekday-hours-${user.id}" min="0" max="24" step="0.5" placeholder="Hours" style="width: 80px; padding: var(--space-2); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-primary); color: var(--text-primary);">
                                    <button type="button" class="btn btn-sm btn-secondary" onclick="setWeekdayHours(${user.id})">Apply</button>
                                </div>
                                <div style="display: flex; align-items: center; gap: var(--space-2);">
                                    <label style="font-weight: 500; white-space: nowrap;">Quick time (Mon-Fri):</label>
                                    <input type="time" id="weekday-start-time-${user.id}" value="07:00" style="padding: var(--space-2); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-primary); color: var(--text-primary);">
                                    <span style="color: var(--text-tertiary);">to</span>
                                    <input type="time" id="weekday-end-time-${user.id}" value="21:00" style="padding: var(--space-2); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-primary); color: var(--text-primary);">
                                    <button type="button" class="btn btn-sm btn-secondary" onclick="setWeekdayTimes(${user.id})">Apply</button>
                                </div>
                            </div>
                        </div>
                        
                        <div class="schedule-grid">
                            ${['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].map((day, index) => {
                                const startTime = schedule.intervals?.[day]?.start_time || '00:00';
                                const endTime = schedule.intervals?.[day]?.end_time || '23:59';
                                return `
                                <div class="schedule-day">
                                    <label for="${day}-${user.id}" class="schedule-day-label">${day.charAt(0).toUpperCase() + day.slice(1)}</label>
                                    <div style="display: flex; flex-direction: column; gap: var(--space-2);">
                                        <div style="display: flex; align-items: center; gap: var(--space-2);">
                                            <input type="number" 
                                                   id="${day}-${user.id}" 
                                                   name="${day}" 
                                                   min="0" 
                                                   max="24" 
                                                   step="0.5"
                                                   value="${schedule.hours[day] || 0}"
                                                   class="schedule-input form-control"
                                                   style="width: 80px;">
                                            <span style="font-size: var(--font-size-xs); color: var(--text-tertiary);">hours</span>
                                        </div>
                                        <div style="display: flex; align-items: center; gap: var(--space-1); font-size: var(--font-size-xs);">
                                            <input type="time" 
                                                   id="${day}-start-time-${user.id}" 
                                                   name="${day}_start_time" 
                                                   value="${startTime}"
                                                   class="form-control"
                                                   style="width: 70px; font-size: var(--font-size-xs); padding: var(--space-1);">
                                            <span style="color: var(--text-tertiary);">to</span>
                                            <input type="time" 
                                                   id="${day}-end-time-${user.id}" 
                                                   name="${day}_end_time" 
                                                   value="${endTime}"
                                                   class="form-control"
                                                   style="width: 70px; font-size: var(--font-size-xs); padding: var(--space-1);">
                                        </div>
                                    </div>
                                </div>
                                `;
                            }).join('')}
                        </div>
                        
                        <div class="schedule-actions" style="display: flex; gap: var(--space-3); margin-top: var(--space-4);">
                            <button type="submit" class="btn btn-primary">
                                <svg width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                                    <path d="M15.854 5.146a.5.5 0 0 1 0 .708l-3 3a.5.5 0 0 1-.708 0l-1.5-1.5a.5.5 0 0 1 .708-.708L12.5 7.793l2.646-2.647a.5.5 0 0 1 .708 0z"/>
                                    <path d="M1 14s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H1zm5-6a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"/>
                                </svg>
                                Save Schedule
                            </button>
                            <button type="button" class="btn btn-secondary" onclick="resetScheduleForm(${user.id})">
                                <svg width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                                    <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2v1z"/>
                                    <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466z"/>
                                </svg>
                                Reset
                            </button>
                        </div>
                        
                        ${user.lastScheduleModified ? `
                            <div style="margin-top: var(--space-4); padding-top: var(--space-4); border-top: 1px solid var(--border-primary); font-size: var(--font-size-xs); color: var(--text-muted);">
                                <div style="display: flex; justify-content: space-between; margin-bottom: var(--space-2);">
                                    <span>Last Modified:</span>
                                    <span>${user.lastScheduleModified}</span>
                                </div>
                                ${user.lastScheduleSync ? `
                                    <div style="display: flex; justify-content: space-between;">
                                        <span>Last Synced:</span>
                                        <span>${user.lastScheduleSync}</span>
                                    </div>
                                ` : ''}
                            </div>
                        ` : ''}
                    </form>
                </div>
            </div>
        `;
    }).join('');
}

// Weekly Schedule helper functions
function setWeekdayHours(userId) {
    const weekdayHours = document.getElementById(`weekday-hours-${userId}`).value;
    if (!weekdayHours) {
        showNotification('Please enter hours for weekdays', 'warning');
        return;
    }
    
    // Set Monday through Friday
    const weekdays = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'];
    weekdays.forEach(day => {
        document.getElementById(`${day}-${userId}`).value = weekdayHours;
    });
}

function setWeekdayTimes(userId) {
    const startTime = document.getElementById(`weekday-start-time-${userId}`).value;
    const endTime = document.getElementById(`weekday-end-time-${userId}`).value;
    
    if (!startTime || !endTime) {
        showNotification('Please select both start and end times', 'warning');
        return;
    }
    
    if (startTime >= endTime) {
        showNotification('Start time must be before end time', 'warning');
        return;
    }
    
    // Set Monday through Friday
    const weekdays = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'];
    weekdays.forEach(day => {
        document.getElementById(`${day}-start-time-${userId}`).value = startTime;
        document.getElementById(`${day}-end-time-${userId}`).value = endTime;
    });
}

function resetScheduleForm(userId) {
    if (confirm('Are you sure you want to reset the schedule to the last saved values?')) {
        // Reload the weekly schedule page to restore original values
        showWeeklySchedule();
    }
}

async function submitScheduleForm(event, userId) {
    event.preventDefault();
    
    const scheduleData = {
        user_id: userId,
        monday: parseFloat(document.getElementById(`monday-${userId}`).value || 0),
        tuesday: parseFloat(document.getElementById(`tuesday-${userId}`).value || 0),
        wednesday: parseFloat(document.getElementById(`wednesday-${userId}`).value || 0),
        thursday: parseFloat(document.getElementById(`thursday-${userId}`).value || 0),
        friday: parseFloat(document.getElementById(`friday-${userId}`).value || 0),
        saturday: parseFloat(document.getElementById(`saturday-${userId}`).value || 0),
        sunday: parseFloat(document.getElementById(`sunday-${userId}`).value || 0),
        // Time intervals for each day
        monday_start_time: document.getElementById(`monday-start-time-${userId}`).value,
        monday_end_time: document.getElementById(`monday-end-time-${userId}`).value,
        tuesday_start_time: document.getElementById(`tuesday-start-time-${userId}`).value,
        tuesday_end_time: document.getElementById(`tuesday-end-time-${userId}`).value,
        wednesday_start_time: document.getElementById(`wednesday-start-time-${userId}`).value,
        wednesday_end_time: document.getElementById(`wednesday-end-time-${userId}`).value,
        thursday_start_time: document.getElementById(`thursday-start-time-${userId}`).value,
        thursday_end_time: document.getElementById(`thursday-end-time-${userId}`).value,
        friday_start_time: document.getElementById(`friday-start-time-${userId}`).value,
        friday_end_time: document.getElementById(`friday-end-time-${userId}`).value,
        saturday_start_time: document.getElementById(`saturday-start-time-${userId}`).value,
        saturday_end_time: document.getElementById(`saturday-end-time-${userId}`).value,
        sunday_start_time: document.getElementById(`sunday-start-time-${userId}`).value,
        sunday_end_time: document.getElementById(`sunday-end-time-${userId}`).value
    };
    
    const result = await apiCall('/schedule/update', 'POST', scheduleData);
    
    if (result && result.success) {
        showNotification(`Schedule updated for user ${userId}`, 'success');
        // Refresh the weekly schedule page to show updated data
        setTimeout(() => {
            showWeeklySchedule();
        }, 1000);
    } else {
        const message = result ? result.message : 'Failed to save schedule';
        showNotification(`Error: ${message}`, 'error');
    }
}

// Initialize theme on page load
function initializeTheme() {
    const savedTheme = localStorage.getItem('theme') || 'light';
    const html = document.documentElement;
    html.setAttribute('data-theme', savedTheme);
    
    const toggle = document.querySelector('.theme-toggle');
    const icon = toggle.querySelector('.theme-toggle-icon path');
    const text = toggle.querySelector('.theme-toggle-text');
    
    if (savedTheme === 'dark') {
        icon.setAttribute('d', 'M6 .278a.768.768 0 0 1 .08.858 7.208 7.208 0 0 0-.878 3.46c0 4.021 3.278 7.277 7.318 7.277.527 0 1.04-.055 1.533-.16a.787.787 0 0 1 .81.316.733.733 0 0 1-.031.893A8.349 8.349 0 0 1 8.344 16C3.734 16 0 12.286 0 7.71 0 4.266 2.114 1.312 5.124.06A.752.752 0 0 1 6 .278z');
        text.textContent = 'Light';
    } else {
        icon.setAttribute('d', 'M8 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM8 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 0zm0 13a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 13zm8-5a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2a.5.5 0 0 1 .5.5zM3 8a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2A.5.5 0 0 1 3 8zm10.657-5.657a.5.5 0 0 1 0 .707l-1.414 1.415a.5.5 0 1 1-.707-.708l1.414-1.414a.5.5 0 0 1 .707 0zm-9.193 9.193a.5.5 0 0 1 0 .707L3.05 13.657a.5.5 0 0 1-.707-.707l1.414-1.414a.5.5 0 0 1 .707 0zm9.193 2.121a.5.5 0 0 1-.707 0l-1.414-1.414a.5.5 0 0 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .707zM4.464 4.465a.5.5 0 0 1-.707 0L2.343 3.05a.5.5 0 1 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .708z');
        text.textContent = 'Dark';
    }
}