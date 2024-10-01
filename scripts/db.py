import os
import supabase

supabase_client = supabase.create_client(
    "https://twyzufmqxsqoaqjidwbu.supabase.co", os.environ.get("SUPABASE_API_KEY")
)
