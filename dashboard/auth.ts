import NextAuth, { type DefaultSession } from "next-auth"
import CredentialsProvider from "next-auth/providers/credentials"
import { PrismaAdapter } from "@auth/prisma-adapter"
import prisma from "./lib/db"
import bcrypt from "bcryptjs"

export type Role = "Org_Owner" | "Org_Admin" | "Employee"

declare module "next-auth" {
  interface Session {
    user: {
      role: Role
      organizationId: string
      id: string
    } & DefaultSession["user"]
  }

  interface User {
    role: Role
    organizationId: string
  }
}

export const { handlers, signIn, signOut, auth } = NextAuth({
  adapter: PrismaAdapter(prisma),
  session: { strategy: "jwt" },
  providers: [
    CredentialsProvider({
      name: "Credentials",
      credentials: {
        email: { label: "Email", type: "email" },
        password: { label: "Password", type: "password" }
      },
      authorize: async (credentials) => {
        const email = credentials.email as string
        const password = credentials.password as string

        const user = await prisma.user.findUnique({
          where: { email },
          include: { memberships: true }
        })
        
        if (!user || !user.passwordHash) {
          throw new Error("Invalid credentials.")
        }

        const isPasswordValid = await bcrypt.compare(password, user.passwordHash)
        if (!isPasswordValid) {
          throw new Error("Invalid credentials.")
        }
        
        const mainMembership = user.memberships[0]
        
        return {
          id: user.id,
          email: user.email,
          name: user.name,
          image: user.image,
          role: mainMembership ? (mainMembership.role as Role) : "Employee",
          organizationId: mainMembership ? mainMembership.organizationId : "",
        }
      }
    })
  ],
  callbacks: {
    jwt({ token, user, trigger, session }) {
      if (user) {
        token.id = user.id
        token.role = user.role
        token.organizationId = user.organizationId
      }
      // Allow updating active organization
      if (trigger === "update" && session?.organizationId) {
        token.organizationId = session.organizationId
        token.role = session.role
      }
      return token
    },
    session({ session, token }) {
      if (token && session.user) {
        session.user.id = token.id as string
        session.user.role = token.role as Role
        session.user.organizationId = token.organizationId as string
      }
      return session
    }
  },
  pages: {
    signIn: "/login",
  }
})
