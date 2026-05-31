CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL,
    role TEXT NOT NULL,
    deleted_at TIMESTAMP DEFAULT NULL,
    password_hash TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS expense_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS claims (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    employee_id INTEGER NOT NULL,
    manager_id INTEGER NOT NULL,
    item TEXT NOT NULL,
    category_id INTEGER NOT NULL,
    remarks TEXT,
    purchase_date TIMESTAMP NOT NULL,
    tax_rate DECIMAL(10, 2) NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    status TEXT NOT NULL,
    manager_remarks TEXT,
    vendor_name TEXT,
    date_submitted TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    date_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (employee_id) REFERENCES users(id),
    FOREIGN KEY (manager_id) REFERENCES users(id),
    FOREIGN KEY (category_id) REFERENCES expense_categories(id)
);
CREATE TABLE IF NOT EXISTS claim_receipts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id INTEGER NOT NULL,
    attachment_url TEXT,
    uploaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (claim_id) REFERENCES claims(id)
);
CREATE TABLE IF NOT EXISTS financial_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    generated_by INTEGER NOT NULL,
    report_type TEXT NOT NULL,
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,
    attachment_url TEXT,
    status TEXT NOT NULL,
    generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (generated_by) REFERENCES users(id)
);
CREATE TABLE IF NOT EXISTS invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id INTEGER NOT NULL,
    invoice_number TEXT NOT NULL,
    FOREIGN KEY (claim_id) REFERENCES claims(id)
);
CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    stock_quantity INTEGER NOT NULL,
    subtotal DECIMAL(10, 2) NOT NULL,
    total_amount DECIMAL(10, 2) NOT NULL,
    FOREIGN KEY (invoice_id) REFERENCES invoices(id)
);
CREATE TABLE IF NOT EXISTS payment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL,
    expense_id INTEGER NOT NULL,
    payment_direction TEXT NOT NULL,
    reference_number TEXT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    payment_method TEXT NOT NULL,
    payment_date TIMESTAMP NOT NULL,
    remarks TEXT,
    attachment TEXT,
    FOREIGN KEY (invoice_id) REFERENCES invoices(id),
    FOREIGN KEY (expense_id) REFERENCES claims(id)
);
CREATE TABLE IF NOT EXISTS crm (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    crm_type TEXT NOT NULL,
    company_name TEXT,
    contact_person TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT,
    address TEXT,
    status TEXT DEFAULT 'Active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT NULL
);
INSERT INTO users (name, email, phone, status, role, password_hash) VALUES
('Alice', 'alice@example.com', '123-456-7890', 'Active', 'Employee', 'hashed_password'),
('Bob', 'bob@example.com', '098-765-4321', 'Active', 'Manager', 'hashed_password');
