## <img alt="security icon" src="./assets/readme/security.svg" height="24" style="vertical-align: middle;"> Security Policy

> [!IMPORTANT]
> This document outlines the security practices and reporting guidelines for Steam Cookie Searcher.

### <img alt="features icon" height="18" src="./assets/readme/features.svg" style="vertical-align: middle;">&nbsp;&nbsp;Supported Versions

Please use the latest version (`main` branch) to ensure you have the most up-to-date security patches. Older releases are not actively maintained.

### <img alt="documentation icon" height="18" src="./assets/readme/documentation.svg" style="vertical-align: middle;">&nbsp;&nbsp;Security & Privacy Model

The application is designed to process data locally and minimize data exposure:

1. **Local Processing:**
   All operations, parsing, and data extractions are processed locally. The application does not transmit cookies, credentials, or session data to any central server or analytics provider.
   
2. **Headless Execution:**
   The `chromedriver` instances are spawned as detached processes without persistent profiles (using arguments like `--incognito`). This ensures no local storage or cache is saved to the disk after the execution ends.

3. **Memory Safety:**
   The application leverages Rust's memory safety guarantees to prevent common issues like buffer overflows when parsing user-provided `.zip` files.

4. **Output Handling:**
   Extracted cookies and JWTs are not printed to `stdout` or logged to background files. The data is only saved to disk if the user explicitly uses the manual export function.

5. **External APIs:**
   When validating Internet Protocol (IP) addresses, the application only contacts geolocation endpoints (e.g., `ip-api.com`). No Steam account data is attached to these proxy requests.

### <img alt="git-branch icon" height="18" src="./assets/readme/git-branch.svg" style="vertical-align: middle;">&nbsp;&nbsp;Reporting a Vulnerability

If you discover a security flaw or bug, please report it through the repository channels:

1. **GitHub Issues:** For general bugs or non-critical flaws, open an issue in the **Issues tab** using the "Bug Report" label. Provide technical details and steps to reproduce.
2. **Private Reporting:** If you find a critical vulnerability (e.g., something that compromises user tokens), please use GitHub's **Security Advisories** feature (found in the `Security` tab) to report it privately before it becomes public.
3. **Response:** We monitor the issue tracker and will patch security-related tickets as quickly as possible.
