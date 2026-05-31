"use server"

import prisma from "@/lib/db"
import { auth } from "@/auth"
import { revalidatePath } from "next/cache"

export async function updateProfile(formData: FormData) {
  const session = await auth()
  if (!session?.user?.id) throw new Error("Unauthorized")

  const name = formData.get("name") as string
  const email = formData.get("email") as string // maybe read-only
  
  // Note: updating avatar image can also be done here, but usually requires S3 upload. 
  // We'll just allow name for now.

  await prisma.user.update({
    where: { id: session.user.id },
    data: { name }
  })

  revalidatePath("/dashboard/settings/profile")
}
