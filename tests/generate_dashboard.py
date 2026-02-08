#!/usr/bin/env python3
"""
Generate HTML dashboard for Clorus test metrics
"""

import json
import sys
import os
from datetime import datetime
from pathlib import Path

def generate_html_dashboard(metrics_file, output_file):
    """Generate an HTML dashboard from metrics JSON"""

    # Read metrics
    with open(metrics_file, 'r') as f:
        metrics = json.load(f)

    summary = metrics['summary']
    categories = metrics['categories']
    timestamp = metrics.get('timestamp', datetime.now().strftime('%Y-%m-%d %H:%M:%S'))

    # Generate HTML
    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Clorus Test Metrics Dashboard</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }}

        .container {{
            max-width: 1200px;
            margin: 0 auto;
        }}

        .header {{
            text-align: center;
            color: white;
            margin-bottom: 40px;
        }}

        .header h1 {{
            font-size: 3em;
            margin-bottom: 10px;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
        }}

        .header .subtitle {{
            font-size: 1.2em;
            opacity: 0.9;
        }}

        .header .timestamp {{
            margin-top: 10px;
            font-size: 0.9em;
            opacity: 0.8;
        }}

        .summary-cards {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }}

        .card {{
            background: white;
            border-radius: 15px;
            padding: 30px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            transition: transform 0.3s ease;
        }}

        .card:hover {{
            transform: translateY(-5px);
        }}

        .card-title {{
            font-size: 0.9em;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 1px;
            margin-bottom: 10px;
        }}

        .card-value {{
            font-size: 3em;
            font-weight: bold;
            margin-bottom: 10px;
        }}

        .card-subtitle {{
            font-size: 0.9em;
            color: #999;
        }}

        .card.success .card-value {{ color: #10b981; }}
        .card.danger .card-value {{ color: #ef4444; }}
        .card.info .card-value {{ color: #3b82f6; }}

        .progress-container {{
            background: white;
            border-radius: 15px;
            padding: 30px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            margin-bottom: 40px;
        }}

        .progress-header {{
            font-size: 1.5em;
            font-weight: bold;
            margin-bottom: 20px;
            color: #333;
        }}

        .progress-bar-container {{
            height: 40px;
            background: #f3f4f6;
            border-radius: 20px;
            overflow: hidden;
            margin-bottom: 30px;
            position: relative;
        }}

        .progress-bar {{
            height: 100%;
            background: linear-gradient(90deg, #10b981 0%, #34d399 100%);
            transition: width 1s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: bold;
            font-size: 1.1em;
        }}

        .category-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
            gap: 20px;
        }}

        .category-card {{
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
        }}

        .category-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
        }}

        .category-name {{
            font-size: 1.2em;
            font-weight: bold;
            color: #333;
        }}

        .category-stats {{
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 10px;
            margin-bottom: 15px;
        }}

        .stat {{
            text-align: center;
            padding: 10px;
            background: #f9fafb;
            border-radius: 8px;
        }}

        .stat-label {{
            font-size: 0.7em;
            color: #999;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}

        .stat-value {{
            font-size: 1.5em;
            font-weight: bold;
            color: #333;
        }}

        .category-progress {{
            height: 10px;
            background: #f3f4f6;
            border-radius: 5px;
            overflow: hidden;
        }}

        .category-progress-bar {{
            height: 100%;
            transition: width 0.5s ease;
        }}

        .progress-excellent {{ background: linear-gradient(90deg, #10b981, #34d399); }}
        .progress-good {{ background: linear-gradient(90deg, #3b82f6, #60a5fa); }}
        .progress-warning {{ background: linear-gradient(90deg, #f59e0b, #fbbf24); }}
        .progress-poor {{ background: linear-gradient(90deg, #ef4444, #f87171); }}

        .badge {{
            display: inline-block;
            padding: 4px 12px;
            border-radius: 12px;
            font-size: 0.85em;
            font-weight: bold;
        }}

        .badge.excellent {{ background: #d1fae5; color: #065f46; }}
        .badge.good {{ background: #dbeafe; color: #1e40af; }}
        .badge.warning {{ background: #fef3c7; color: #92400e; }}
        .badge.poor {{ background: #fee2e2; color: #991b1b; }}

        @media (max-width: 768px) {{
            .header h1 {{ font-size: 2em; }}
            .summary-cards {{ grid-template-columns: 1fr; }}
            .category-grid {{ grid-template-columns: 1fr; }}
        }}

        .footer {{
            text-align: center;
            color: white;
            margin-top: 40px;
            padding: 20px;
            opacity: 0.8;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🧪 Clorus Test Metrics</h1>
            <div class="subtitle">Comprehensive Test Coverage Dashboard</div>
            <div class="timestamp">Generated: {timestamp}</div>
        </div>

        <div class="summary-cards">
            <div class="card info">
                <div class="card-title">Total Tests</div>
                <div class="card-value">{summary['total_tests']}</div>
                <div class="card-subtitle">Test cases executed</div>
            </div>

            <div class="card success">
                <div class="card-title">Passed</div>
                <div class="card-value">{summary['passed']}</div>
                <div class="card-subtitle">{summary['pass_rate']}% success rate</div>
            </div>

            <div class="card danger">
                <div class="card-title">Failed</div>
                <div class="card-value">{summary['failed']}</div>
                <div class="card-subtitle">{100 - summary['pass_rate']:.1f}% failure rate</div>
            </div>
        </div>

        <div class="progress-container">
            <div class="progress-header">Overall Test Coverage</div>
            <div class="progress-bar-container">
                <div class="progress-bar" style="width: {summary['pass_rate']}%">
                    {summary['pass_rate']}%
                </div>
            </div>
        </div>

        <div class="category-grid">
"""

    # Add category cards
    for category_name, category_data in sorted(categories.items()):
        total = category_data['total']
        passed = category_data['passed']
        failed = category_data['failed']
        rate = category_data['pass_rate']

        # Determine badge class
        if rate >= 90:
            badge_class = "excellent"
            progress_class = "progress-excellent"
        elif rate >= 70:
            badge_class = "good"
            progress_class = "progress-good"
        elif rate >= 50:
            badge_class = "warning"
            progress_class = "progress-warning"
        else:
            badge_class = "poor"
            progress_class = "progress-poor"

        html += f"""
            <div class="category-card">
                <div class="category-header">
                    <div class="category-name">{category_name.title()}</div>
                    <span class="badge {badge_class}">{rate:.1f}%</span>
                </div>

                <div class="category-stats">
                    <div class="stat">
                        <div class="stat-label">Total</div>
                        <div class="stat-value">{total}</div>
                    </div>
                    <div class="stat">
                        <div class="stat-label">Passed</div>
                        <div class="stat-value" style="color: #10b981;">{passed}</div>
                    </div>
                    <div class="stat">
                        <div class="stat-label">Failed</div>
                        <div class="stat-value" style="color: #ef4444;">{failed}</div>
                    </div>
                </div>

                <div class="category-progress">
                    <div class="category-progress-bar {progress_class}" style="width: {rate}%"></div>
                </div>
            </div>
"""

    html += """
        </div>

        <div class="footer">
            <p>Clorus Programming Language - Test Metrics Dashboard</p>
            <p style="font-size: 0.9em; margin-top: 10px;">
                Built with ❤️ for reliable software development
            </p>
        </div>
    </div>

    <script>
        // Animate progress bars on load
        window.addEventListener('load', function() {
            const bars = document.querySelectorAll('.progress-bar, .category-progress-bar');
            bars.forEach(bar => {
                const width = bar.style.width;
                bar.style.width = '0';
                setTimeout(() => {
                    bar.style.width = width;
                }, 100);
            });
        });
    </script>
</body>
</html>
"""

    # Write HTML file
    with open(output_file, 'w') as f:
        f.write(html)

    print(f"✓ HTML dashboard generated: {output_file}")
    print(f"  Open in browser: file://{os.path.abspath(output_file)}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python3 generate_dashboard.py <metrics.json> [output.html]")
        sys.exit(1)

    metrics_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else 'test-metrics/dashboard.html'

    if not os.path.exists(metrics_file):
        print(f"Error: Metrics file not found: {metrics_file}")
        sys.exit(1)

    # Create output directory if needed
    output_dir = os.path.dirname(output_file)
    if output_dir and not os.path.exists(output_dir):
        os.makedirs(output_dir)

    generate_html_dashboard(metrics_file, output_file)
