# HR Module

## Overview

The HR module provides comprehensive human resources management including employee records, departments, positions, payroll, attendance, and performance reviews.

## Functionality

### Organization Management
- Department hierarchy (tree structure)
- Position management
- Reporting lines

### Employee Management
- Employee profiles
- Personal information
- Employment details
- Contract types
- Termination handling

### Payroll
- Salary calculation
- Payroll processing
- Salary payments
- Payroll approval workflow
- Salary history

### Attendance
- Check-in/check-out tracking
- Attendance reports
- Overtime calculation

### Leave Management
- Leave request workflow
- Leave types (annual, sick, unpaid, etc.)
- Leave balance tracking
- Approval process

### Performance
- Performance reviews
- Rating system
- Review history

## API Endpoints

### Departments
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/hr/departments` | List departments |
| GET | `/api/hr/departments/tree` | Department hierarchy |
| POST | `/api/hr/departments` | Create department |
| PUT | `/api/hr/departments/{id}` | Update department |
| DELETE | `/api/hr/departments/{id}` | Delete department |

### Positions
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/hr/positions` | List positions |
| POST | `/api/hr/positions` | Create position |
| PUT | `/api/hr/positions/{id}` | Update position |
| DELETE | `/api/hr/positions/{id}` | Delete position |

### Employees
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/hr/employees` | List employees |
| GET | `/api/hr/employees/{id}` | Get employee |
| POST | `/api/hr/employees` | Create employee |
| PUT | `/api/hr/employees/{id}` | Update employee |
| DELETE | `/api/hr/employees/{id}` | Delete employee |

### Payroll
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/hr/payroll` | List payroll records |
| POST | `/api/hr/payroll/calculate` | Calculate payroll |
| POST | `/api/hr/payroll` | Create payroll |
| POST | `/api/hr/payroll/{id}/approve` | Approve payroll |

### Attendance
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/hr/attendance/check-in` | Check in |
| POST | `/api/hr/attendance/check-out` | Check out |
| GET | `/api/hr/attendance` | Attendance records |
| GET | `/api/hr/attendance/report` | Attendance report |

### Leave
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/hr/leave/requests` | Leave requests |
| POST | `/api/hr/leave/requests` | Submit leave request |
| POST | `/api/hr/leave/requests/{id}/approve` | Approve request |
| GET | `/api/hr/leave/balances` | Leave balances |

### Performance
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/hr/performance` | Performance reviews |
| POST | `/api/hr/performance` | Create review |
| PUT | `/api/hr/performance/{id}` | Update review |

## Data Model

### Department
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Department code |
| name | String | Department name |
| parent_id | Integer | Parent department |
| manager_id | Integer | Manager employee |
| is_active | Boolean | Active status |

### Employee
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| employee_number | String | Employee ID |
| first_name | String | First name |
| last_name | String | Last name |
| department_id | Integer | Department |
| position_id | Integer | Position |
| hire_date | Date | Hire date |
| termination_date | Date | Termination date |
| salary | Decimal | Base salary |
| status | Enum | active, terminated |

## Example Usage

```bash
# Create department
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/hr/departments" \
  -d '{"code": "IT", "name": "Information Technology"}'

# Create employee
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/hr/employees" \
  -d '{
    "employee_number": "EMP001",
    "first_name": "John",
    "last_name": "Doe",
    "department_id": 1,
    "position_id": 1,
    "hire_date": "2024-01-01",
    "salary": 5000.00
  }'

# Check in
curl -X POST -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/hr/attendance/check-in"
```
