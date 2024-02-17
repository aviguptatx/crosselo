import os
import supabase

supabase_client = supabase.create_client(
    os.environ.get("SUPABASE_API_URL"), os.environ.get("SUPABASE_API_KEY")
)
