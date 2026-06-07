# CSC1106 Web Programming Project: Rust Web Engineering and Enterprise Application Development

## Background

Design and develop a modern enterprise web application using **Rust** and **Actix Web**. The project simulates real-world business software (finance, healthcare, education, service industries). Apply OOP principles (structs, impl blocks, traits), server-side rendering (Tera), database integration, and business workflow implementation.

## Intended Learning Outcomes

- Design and develop enterprise web applications using Rust & Actix Web
- Apply OOP concepts, modular architecture, SSR, and relational database integration
- Analyze business requirements through literature/background study
- Implement technically complex features (algorithms, concurrency, scheduling, validation, recommendation)
- Strengthen collaborative engineering and the ability to justify design decisions

## Project Domain Selection

Select **ONE** domain. Each has a **required core engineering focus** beyond basic modules.

### 1. Accounting System

**Modules:** Customer Management, Invoice Management, Expense Tracking, Financial Reporting, Tax Calculation, Payment Records

**Required core:** Double Entry Accounting Engine — every transaction must maintain balanced debit/credit records. Implement transaction posting workflows, ledger balancing, and audit trail generation.

**Advanced:** Monthly balance sheets, dashboard analytics, transaction history, PDF invoice export

### 2. Patient Management System

**Modules:** Patient Registration, Appointment Scheduling, Medical Records, Billing, Doctor Management, Prescription Tracking

**Required core:** Appointment Scheduling & Conflict Resolution — schedule doctors, patients, and facilities without overlaps or resource conflicts. Implement scheduling algorithms, queue management, and time slot validation.

**Advanced:** Appointment queue prioritization, patient history timelines, medical report generation, role-based staff access

### 3. Banking System

**Modules:** User Account Management, Money Transfer, Transaction History, Loan Management, Fixed Deposit, Admin Dashboard

**Required core:** Concurrency Safe Money Transfer Engine — prevent race conditions, inconsistent balances, and double spending during simultaneous transactions. Explore thread safety, Mutex locking, transactional consistency, and rollback mechanisms.

**Advanced:** Transaction audit logging, OTP simulation, fraud detection rules, secure transaction verification

### 4. Learning Management Portal (LMS)

**Modules:** Course Management, Student Enrollment, Assignment Submission, Gradebook, Attendance Tracking, Discussion Forums

**Required core:** Adaptive Quiz & Recommendation Engine — dynamically adjust quiz difficulty and recommend learning materials based on student performance. Design scoring algorithms, learning analytics, and rule-based recommendation systems.

**Advanced:** Instructor dashboards, analytics reporting, file uploads, personalized learning pathways

---

## Literature & Background Study

Before implementation, conduct a brief literature review: explore existing systems, technical articles, academic papers, and industry case studies for your chosen domain. Identify common business processes, user roles, data relationships, security considerations, and advanced features. This informs realistic, meaningful system design.

---

## Marking Criteria

### Group Project Implementation (60%)

| #   | Criteria                                    | Weight |
| --- | ------------------------------------------- | ------ |
| 1   | System Architecture & OOP Design            | 15%    |
| 2   | Backend Functionality & Business Logic      | 15%    |
| 3   | Database Design & Integration               | 10%    |
| 4   | Frontend Design & Server-Side Rendering     | 10%    |
| 5   | Documentation, Presentation & Demonstration | 10%    |

**Excellent** = well-structured, robust, professional; **Average** = functional but lacks depth/consistency; **Poor** = incomplete, disorganized, minimal understanding.

### Individual Extended Features & Technical Complexity (40%)

| #   | Criteria                                | Weight |
| --- | --------------------------------------- | ------ |
| 1   | Extended Feature Development            | 15%    |
| 2   | Technical Complexity & Problem Solving  | 15%    |
| 3   | Individual Understanding & Contribution | 10%    |

Each student must individually own advanced features, enhancements, or system complexity beyond the group baseline.

**Group vs Individual:** Group mark = collaborative delivery (architecture, DB, frontend, backend, overall functionality). Individual mark = your independent technical contribution beyond the shared baseline.

---

## Group & Peer Evaluation

Groups must engage in regular, open discussion. Peer evaluation is required — submit before the project deadline in case of disputation.

---

## Submission Requirements

Group leader ensures all materials are submitted on time to **xSiTe Dropbox** using the specified naming conventions. All files must display: **Group Number**, **Student Name(s)**, **Student ID(s) (SIT)**.

File naming: `g##` = group number (e.g., `g03`, `g22`). All lowercase.

| Deliverable                                                                                        | Format                      | Max Size |
| -------------------------------------------------------------------------------------------------- | --------------------------- | -------- |
| Source Code Archive                                                                                | `g##_source.zip`            | 20 MB    |
| Demonstration Recording (15 min, original speed, every member demos their contribution)            | `g##_recording.mp4`         | 200 MB   |
| Presentation Slides — source                                                                       | `g##_slides.ppt` or `.pptx` | 20 MB    |
| Presentation Slides — PDF                                                                          | `g##_slides.pdf`            | 20 MB    |
| Project Report (max 6 pages, each member states group contribution + individual features) — source | `g##_report.doc` or `.docx` | 20 MB    |
| Project Report — PDF                                                                               | `g##_report.pdf`            | 20 MB    |

---

## Formative Assessment (Conditional — 10%)

> **Does not currently apply.** Only takes effect after official confirmation from the class representative.

If activated: submit a max 2-page document by end of **Week 5** (feedback by end of **Week 6**) covering proposed system functionality, business workflows, domain, module planning, and intended advanced features.

| Criteria                                       | Weight |
| ---------------------------------------------- | ------ |
| Project Direction & Business Workflow Planning | 4%     |
| Feature Scope & Complexity Planning            | 3%     |
| Literature Review & Background Study           | 3%     |

---

## Use of Generative AI

**Strictly prohibited** for generating source code, reports, documentation, or submission materials. AI may only be used in a supporting role (clarifying concepts, background research, understanding docs). Academic integrity policies apply fully.

## Hints

Focus on **computing science depth** — architecture, algorithms, concurrency, validation, database integration, business workflows, security, maintainable code — over visually attractive but technically superficial features. Review lecture materials, tutorials, and official framework documentation.
