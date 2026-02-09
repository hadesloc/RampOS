"use client";

import { useState } from "react";
import { Bell } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";

interface Notification {
  id: string;
  title: string;
  description: string;
  date: string;
  read: boolean;
  type: 'system' | 'alert' | 'info';
}

const mockNotifications: Notification[] = [
  {
    id: '1',
    title: 'System Maintenance',
    description: 'Scheduled maintenance on Sunday at 2 AM UTC.',
    date: '2 hours ago',
    read: false,
    type: 'system',
  },
  {
    id: '2',
    title: 'High Volume Alert',
    description: 'Unusual spike in pay-in volume detected.',
    date: '5 hours ago',
    read: false,
    type: 'alert',
  },
  {
    id: '3',
    title: 'New Feature Available',
    description: 'Check out the new compliance reporting tools.',
    date: '1 day ago',
    read: true,
    type: 'info',
  },
];

export function NotificationCenter() {
  const [notifications, setNotifications] = useState<Notification[]>(mockNotifications);
  const unreadCount = notifications.filter(n => !n.read).length;

  const markAllAsRead = () => {
    setNotifications(notifications.map(n => ({ ...n, read: true })));
  };

  const NotificationList = ({ items }: { items: Notification[] }) => (
    <ScrollArea className="h-[300px] w-full">
      {items.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-full p-4 text-muted-foreground text-sm">
          <Bell className="h-8 w-8 mb-2 opacity-20" />
          No notifications
        </div>
      ) : (
        <div className="flex flex-col">
          {items.map((notification) => (
            <div
              key={notification.id}
              className={`flex flex-col gap-1 p-4 hover:bg-muted/50 transition-colors ${!notification.read ? 'bg-muted/20' : ''}`}
            >
              <div className="flex justify-between items-start">
                <h4 className={`text-sm font-semibold ${!notification.read ? 'text-foreground' : 'text-muted-foreground'}`}>
                  {notification.title}
                </h4>
                {!notification.read && (
                  <span className="h-2 w-2 rounded-full bg-blue-500 mt-1" />
                )}
              </div>
              <p className="text-xs text-muted-foreground line-clamp-2">
                {notification.description}
              </p>
              <span className="text-[10px] text-muted-foreground/60 mt-1">
                {notification.date}
              </span>
              <Separator className="mt-4" />
            </div>
          ))}
        </div>
      )}
    </ScrollArea>
  );

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon" className="relative">
          <Bell className="h-5 w-5" />
          {unreadCount > 0 && (
            <span className="absolute top-1.5 right-1.5 h-2 w-2 rounded-full bg-red-500 ring-2 ring-background" />
          )}
          <span className="sr-only">Toggle notifications</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="end">
        <div className="flex items-center justify-between p-4 border-b">
          <h3 className="font-semibold text-sm">Notifications</h3>
          {unreadCount > 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="text-xs h-auto py-1 px-2"
              onClick={markAllAsRead}
            >
              Mark all read
            </Button>
          )}
        </div>
        <Tabs defaultValue="all" className="w-full">
          <TabsList className="w-full justify-start rounded-none border-b bg-transparent p-0">
            <TabsTrigger
              value="all"
              className="relative h-9 rounded-none border-b-2 border-b-transparent bg-transparent px-4 pb-3 pt-2 font-semibold text-muted-foreground shadow-none transition-none data-[state=active]:border-b-primary data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              All
            </TabsTrigger>
            <TabsTrigger
              value="alerts"
              className="relative h-9 rounded-none border-b-2 border-b-transparent bg-transparent px-4 pb-3 pt-2 font-semibold text-muted-foreground shadow-none transition-none data-[state=active]:border-b-primary data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              Alerts
            </TabsTrigger>
            <TabsTrigger
              value="system"
              className="relative h-9 rounded-none border-b-2 border-b-transparent bg-transparent px-4 pb-3 pt-2 font-semibold text-muted-foreground shadow-none transition-none data-[state=active]:border-b-primary data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              System
            </TabsTrigger>
          </TabsList>
          <TabsContent value="all" className="m-0 border-0">
            <NotificationList items={notifications} />
          </TabsContent>
          <TabsContent value="alerts" className="m-0 border-0">
            <NotificationList items={notifications.filter(n => n.type === 'alert')} />
          </TabsContent>
          <TabsContent value="system" className="m-0 border-0">
            <NotificationList items={notifications.filter(n => n.type === 'system')} />
          </TabsContent>
        </Tabs>
      </PopoverContent>
    </Popover>
  );
}
