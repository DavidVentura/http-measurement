import os
import requests

regions = [
    "ams",
    "arn",
    "atl",
    "bog",
    "bom",
    "bos",
    "cdg",
    "den",
    "dfw",
    "ewr",
    "eze",
    "fra",
    "gdl",
    "gig",
    "gru",
    "hkg",
    "iad",
    "jnb",
    "lax",
    "lhr",
    "mad",
    "mia",
    "nrt",
    "ord",
    "otp",
    "phx",
    "qro",
    "scl",
    "sea",
    "sin",
    "sjc",
    "syd",
    "waw",
    "yul",
    "yyz",
]


def create_fly_machine(region: str):
    # Get environment variables
    api_token = os.environ["FLY_API_TOKEN"]
    api_hostname = os.getenv("FLY_API_HOSTNAME", "https://api.machines.dev")
    app_name = os.environ["FLY_APP_NAME"]
    url = f"{api_hostname}/v1/apps/{app_name}/machines"

    headers = {"Authorization": f"Bearer {api_token}", "Content-Type": "application/json"}

    payload = {
        "config": {
            "image": os.environ["FLY_IMAGE"],
            # "init": {"exec": ["/http-measurement", "geo.blog.davidv.dev"]},
            "auto_destroy": True,
            "restart": {"policy": "no"},
            "guest": {"cpu_kind": "shared", "cpus": 1, "memory_mb": 256},
            "skip_service_registration": True,
            "skip_launch": False,
            "region": region,
            "autostart": True,
            "env": {
                "COLLECTION_URL": os.environ["COLLECTION_URL"],
            },
        }
    }

    response = requests.post(url, headers=headers, json=payload)

    # Print status code and response
    print(f"Status Code: {response.status_code}")
    print("Response Headers:")
    for key, value in response.headers.items():
        print(f"{key}: {value}")
    print("\nResponse Body:")
    print(response.json())

    # Raise exception for bad status codes
    response.raise_for_status()

    return response.json()


def main():
    for region in regions:
        try:
            result = create_fly_machine(region)
        except requests.exceptions.RequestException as e:
            print(f"Error: {e}")
        except ValueError as e:
            print(f"Configuration Error: {e}")

if __name__ == "__main__":
    main()
